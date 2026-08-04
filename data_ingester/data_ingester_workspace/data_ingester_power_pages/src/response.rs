use std::fmt::Debug;

use anyhow::Result;
use data_ingester_splunk::splunk::{HecEvent, ToHecEvents};
use itertools::Itertools;
use serde::Serialize;
use serde_json::{json, Value};
use tracing::warn;

use crate::metadata::{DataType, PowerPagesMetadata};
use crate::models::{EnvironmentResponse, ScanRule, SiteSecurityResult, WebsiteDto};
use crate::{SOURCETYPE, SSPHP_RUN_KEY};

const MAX_RESPONSE_BODY: usize = 32_768;

pub fn http_status_success(status: u16) -> bool {
    (200..300).contains(&status)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CollectionOutcome {
    Success,
    NoScan,
    NoData,
    NoPowerPagesSites,
    HttpError,
    ParseError,
    TransportError,
    PreflightFailed,
}

impl CollectionOutcome {
    pub fn from_http(status: u16, is_deep_scan: bool, is_preflight: bool) -> Self {
        if is_preflight && matches!(status, 401 | 403) {
            return Self::PreflightFailed;
        }
        if is_deep_scan && status == 404 {
            return Self::NoScan;
        }
        if http_status_success(status) {
            Self::Success
        } else {
            Self::HttpError
        }
    }
}

#[derive(Debug, Clone)]
pub struct PowerPagesApiResult {
    pub ssphp_http_status: u16,
    pub ssphp_collection_outcome: CollectionOutcome,
    pub response_body: Option<String>,
    pub error_message: Option<String>,
    pub data: Option<Value>,
    pub metadata: PowerPagesMetadata,
}

impl PowerPagesApiResult {
    pub fn environments(&self) -> Vec<EnvironmentResponse> {
        self.items_from_value()
    }

    pub fn websites(&self) -> Vec<WebsiteDto> {
        self.items_from_value()
    }

    pub fn hostnames(&self) -> Vec<String> {
        match &self.data {
            Some(Value::Array(items)) => items
                .iter()
                .filter_map(|v| v.as_str().map(str::to_owned))
                .collect(),
            _ => vec![],
        }
    }

    pub fn waf_status(&self) -> Option<String> {
        match &self.data {
            Some(Value::String(s)) => Some(s.clone()),
            Some(obj) => obj
                .get("status")
                .or_else(|| obj.get("Status"))
                .and_then(|v| v.as_str())
                .map(str::to_owned),
            _ => None,
        }
    }

    fn items_from_value<T: serde::de::DeserializeOwned>(&self) -> Vec<T> {
        let Some(data) = &self.data else {
            return vec![];
        };
        if let Ok(items) = serde_json::from_value::<Vec<T>>(data.clone()) {
            return items;
        }
        if let Some(value) = data.get("value") {
            if let Ok(items) = serde_json::from_value::<Vec<T>>(value.clone()) {
                return items;
            }
        }
        vec![]
    }

    fn truncate_body(body: String) -> String {
        if body.len() <= MAX_RESPONSE_BODY {
            body
        } else {
            format!("{}...[truncated]", &body[..MAX_RESPONSE_BODY])
        }
    }

    fn build_event_payload(&self, mut payload: Value) -> Result<Value> {
        if let Some(obj) = payload.as_object_mut() {
            let _ = obj.insert("ssphp_http_status".into(), json!(self.ssphp_http_status));
            let _ = obj.insert(
                "ssphp_collection_outcome".into(),
                json!(self.ssphp_collection_outcome),
            );
            let _ = obj.insert("metadata".into(), serde_json::to_value(&self.metadata)?);
            if let Some(msg) = &self.error_message {
                let _ = obj.insert("error_message".into(), json!(msg));
            }
        }
        Ok(payload)
    }

    fn make_hec_event(&self, payload: &Value, source: &str) -> Result<HecEvent> {
        HecEvent::new_with_ssphp_run(payload, source, SOURCETYPE, self.get_ssphp_run())
    }

    fn diagnostic_event(&self) -> Result<Vec<HecEvent>> {
        let payload = json!({
            "ssphp_http_status": self.ssphp_http_status,
            "ssphp_collection_outcome": self.ssphp_collection_outcome,
            "response_body": self.response_body,
            "error_message": self.error_message,
            "metadata": self.metadata,
        });
        Ok(vec![self.make_hec_event(&payload, &self.metadata.source())?])
    }

    fn split_environments(&self) -> Result<Vec<HecEvent>> {
        let environments: Vec<EnvironmentResponse> = self.environments();
        if environments.is_empty() && self.ssphp_collection_outcome == CollectionOutcome::Success {
            return self.empty_no_data();
        }
        environments
            .iter()
            .map(|env| {
                let mut meta = PowerPagesMetadata::for_environment(env);
                meta.request_url = self.metadata.request_url.clone();
                let payload = self.build_event_payload(serde_json::to_value(env)?)?;
                self.make_hec_event(&payload, &meta.source())
            })
            .try_collect()
    }

    fn split_websites(&self) -> Result<Vec<HecEvent>> {
        let websites: Vec<WebsiteDto> = self.websites();
        if websites.is_empty() {
            if self.ssphp_collection_outcome == CollectionOutcome::NoPowerPagesSites {
                let payload = self.build_event_payload(json!({
                    "message": "No Power Pages websites in environment",
                }))?;
                return Ok(vec![self.make_hec_event(&payload, &self.metadata.source())?]);
            }
            if self.ssphp_collection_outcome == CollectionOutcome::Success {
                return self.empty_no_data();
            }
        }
        websites
            .iter()
            .map(|site| {
                let meta = PowerPagesMetadata::for_website(
                    &EnvironmentResponse {
                        id: self.metadata.environment_id.clone().unwrap_or_default(),
                        display_name: self.metadata.environment_name.clone().unwrap_or_default(),
                        state: None,
                        r#type: None,
                        tenant_id: None,
                        geo: None,
                        url: None,
                    },
                    site,
                    DataType::Website,
                    &self.metadata.request_url,
                );
                let payload = self.build_event_payload(serde_json::to_value(site)?)?;
                self.make_hec_event(&payload, &meta.source())
            })
            .try_collect()
    }

    fn split_string_array(&self, field: &str) -> Result<Vec<HecEvent>> {
        let hostnames = self.hostnames();
        if hostnames.is_empty() && self.ssphp_collection_outcome == CollectionOutcome::Success {
            return self.empty_no_data();
        }
        hostnames
            .iter()
            .map(|hostname| {
                let payload = self.build_event_payload(json!({ field: hostname }))?;
                self.make_hec_event(&payload, &self.metadata.source())
            })
            .try_collect()
    }

    fn split_top_level_array(&self) -> Result<Vec<HecEvent>> {
        let Some(Value::Array(items)) = &self.data else {
            if self.ssphp_collection_outcome == CollectionOutcome::Success {
                return self.empty_no_data();
            }
            return self.diagnostic_event();
        };
        if items.is_empty() && self.ssphp_collection_outcome == CollectionOutcome::Success {
            return self.empty_no_data();
        }
        items
            .iter()
            .map(|item| {
                let payload = self.build_event_payload(item.clone())?;
                self.make_hec_event(&payload, &self.metadata.source())
            })
            .try_collect()
    }

    fn split_deep_scan_rules(&self) -> Result<Vec<HecEvent>> {
        let Some(data) = &self.data else {
            return self.diagnostic_event();
        };
        let scan: SiteSecurityResult = match serde_json::from_value(data.clone()) {
            Ok(v) => v,
            Err(err) => {
                warn!(error=?err, "Failed parsing deep scan report");
                return self.diagnostic_event();
            }
        };
        if scan.rules.is_empty() && self.ssphp_collection_outcome == CollectionOutcome::Success {
            return self.empty_no_data();
        }
        scan.rules
            .iter()
            .map(|rule| self.deep_scan_rule_event(&scan, rule))
            .try_collect()
    }

    fn deep_scan_rule_event(&self, scan: &SiteSecurityResult, rule: &ScanRule) -> Result<HecEvent> {
        let payload = self.build_event_payload(json!({
            "start_time": scan.start_time,
            "end_time": scan.end_time,
            "failed_rule_count": scan.failed_rule_count,
            "total_alert_count": scan.total_alert_count,
            "total_rule_count": scan.total_rule_count,
            "user_name": scan.user_name,
            "rule": rule,
        }))?;
        self.make_hec_event(&payload, &self.metadata.source())
    }

    fn split_waf_rules(&self) -> Result<Vec<HecEvent>> {
        let Some(data) = &self.data else {
            return self.diagnostic_event();
        };
        let custom = data.get("CustomRules").and_then(|v| v.as_array());
        let managed = data.get("ManagedRules").and_then(|v| v.as_array());
        let mut events = vec![];

        if let Some(rules) = custom {
            for rule in rules {
                let mut meta = self.metadata.clone();
                meta.data_type = DataType::WafCustomRule;
                let mut payload = rule.clone();
                if let Some(obj) = payload.as_object_mut() {
                    let _ = obj.insert("waf_rule_kind".into(), json!("custom"));
                }
                let payload = self.build_event_payload(payload)?;
                events.push(self.make_hec_event(&payload, &meta.source())?);
            }
        }

        if let Some(rule_sets) = managed {
            for rule_set in rule_sets {
                let mut meta = self.metadata.clone();
                meta.data_type = DataType::WafManagedRuleSet;
                let mut payload = rule_set.clone();
                if let Some(obj) = payload.as_object_mut() {
                    let _ = obj.insert("waf_rule_kind".into(), json!("managed_rule_set"));
                }
                let payload = self.build_event_payload(payload)?;
                events.push(self.make_hec_event(&payload, &meta.source())?);

                if let Some(overrides) = rule_set
                    .get("RuleGroupOverrides")
                    .and_then(|v| v.as_array())
                {
                    for group in overrides {
                        let group_name = group
                            .get("RuleGroupName")
                            .and_then(|v| v.as_str())
                            .unwrap_or("");
                        if let Some(rules) = group.get("Rules").and_then(|v| v.as_array()) {
                            for rule in rules {
                                let mut meta = self.metadata.clone();
                                meta.data_type = DataType::WafManagedRuleOverride;
                                let payload = self.build_event_payload(json!({
                                    "waf_rule_kind": "managed_rule_override",
                                    "rule_group_name": group_name,
                                    "rule_set_type": rule_set.get("RuleSetType"),
                                    "rule_set_version": rule_set.get("RuleSetVersion"),
                                    "rule": rule,
                                }))?;
                                events.push(self.make_hec_event(&payload, &meta.source())?);
                            }
                        }
                    }
                }
            }
        }

        if events.is_empty() && self.ssphp_collection_outcome == CollectionOutcome::Success {
            return self.empty_no_data();
        }
        Ok(events)
    }

    fn single_object_event(&self) -> Result<Vec<HecEvent>> {
        let payload = self.build_event_payload(self.data.clone().unwrap_or(json!({})))?;
        Ok(vec![self.make_hec_event(&payload, &self.metadata.source())?])
    }

    fn empty_no_data(&self) -> Result<Vec<HecEvent>> {
        let mut result = self.clone();
        result.ssphp_collection_outcome = CollectionOutcome::NoData;
        result.diagnostic_event()
    }
}

impl ToHecEvents for &PowerPagesApiResult {
    type Item = Value;

    fn source(&self) -> &str {
        "unused"
    }

    fn sourcetype(&self) -> &str {
        SOURCETYPE
    }

    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        Box::new(std::iter::empty())
    }

    fn ssphp_run_key(&self) -> &str {
        SSPHP_RUN_KEY
    }

    fn to_hec_events(&self) -> Result<Vec<HecEvent>> {
        if self.ssphp_collection_outcome != CollectionOutcome::Success
            && self.ssphp_collection_outcome != CollectionOutcome::NoData
            && self.ssphp_collection_outcome != CollectionOutcome::NoScan
            && self.ssphp_collection_outcome != CollectionOutcome::NoPowerPagesSites
        {
            return self.diagnostic_event();
        }

        match self.metadata.data_type {
            DataType::Environment | DataType::Preflight => self.split_environments(),
            DataType::Website => self.split_websites(),
            DataType::Hostname => self.split_string_array("hostname"),
            DataType::SslBinding | DataType::AllowedIp | DataType::Certificate => {
                self.split_top_level_array()
            }
            DataType::DeepScanReport => self.split_deep_scan_rules(),
            DataType::DeepScanScore
            | DataType::WafStatus
            | DataType::RunSummary
            | DataType::WafInactive => self.single_object_event(),
            DataType::WafCustomRule
            | DataType::WafManagedRuleSet
            | DataType::WafManagedRuleOverride => self.split_waf_rules(),
            DataType::NoPowerPagesSites => self.diagnostic_event(),
            DataType::Diagnostic => self.diagnostic_event(),
        }
    }
}

impl PowerPagesApiResult {
    pub fn parse_error_message(body: &str) -> Option<String> {
        serde_json::from_str::<Value>(body).ok().and_then(|v| {
            v.get("error")
                .and_then(|e| e.get("message"))
                .and_then(|m| m.as_str())
                .map(str::to_owned)
        })
    }

    pub fn from_http(
        status: u16,
        body: String,
        metadata: PowerPagesMetadata,
        parsed: Option<Value>,
    ) -> Self {
        let is_preflight = metadata.data_type == DataType::Preflight;
        let mut outcome = CollectionOutcome::from_http(status, metadata.is_deep_scan, is_preflight);
        if http_status_success(status) && parsed.is_none() {
            outcome = CollectionOutcome::ParseError;
        }
        if metadata.data_type == DataType::NoPowerPagesSites {
            outcome = CollectionOutcome::NoPowerPagesSites;
        }
        let error_message = Self::parse_error_message(&body);
        let response_body = if outcome == CollectionOutcome::Success
            || outcome == CollectionOutcome::NoData
            || outcome == CollectionOutcome::NoScan
        {
            None
        } else {
            Some(Self::truncate_body(body))
        };
        Self {
            ssphp_http_status: status,
            ssphp_collection_outcome: outcome,
            response_body,
            error_message,
            data: parsed,
            metadata,
        }
    }
}
