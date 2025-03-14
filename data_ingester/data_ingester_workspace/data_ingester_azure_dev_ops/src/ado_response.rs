use anyhow::{anyhow, Result};
use data_ingester_splunk::splunk::ToHecEvents;
use itertools::Itertools;
use reqwest::header::HeaderMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::trace;

use crate::{
    ado_metadata::{AdoMetadata, AdoMetadataTrait},
    SSPHP_RUN_KEY,
};

/// https://learn.microsoft.com/en-us/azure/devops/integrate/concepts/rate-limits?view=azure-devops
#[derive(Debug, Deserialize)]
pub(crate) struct AdoRateLimiting {
    #[allow(unused)]
    #[serde(rename = "Retry-After")]
    retry_after: usize,

    #[allow(unused)]
    #[serde(rename = "X-RateLimit-Resource")]
    rate_limit_resource: String,

    #[allow(unused)]
    #[serde(rename = "X-RateLimit-Delay")]
    rate_limit_delay: usize,

    #[allow(unused)]
    #[serde(rename = "X-RateLimit-Limit")]
    rate_limit_limit: usize,

    #[allow(unused)]
    #[serde(rename = "X-RateLimit-Remaining")]
    rate_limit_remaining: usize,

    #[allow(unused)]
    #[serde(rename = "X-RateLimit-Reset")]
    // Uknown type
    rate_limit_reset: usize,
}

impl AdoRateLimiting {
    fn get_usize_from_header(headers: &HeaderMap, key: &str) -> usize {
        static DEFAULT: usize = 0;
        headers
            .get(key)
            .map(|h| {
                h.to_str()
                    .unwrap_or_default()
                    .parse::<usize>()
                    .unwrap_or(DEFAULT)
            })
            .unwrap_or(DEFAULT)
    }

    pub(crate) fn from_headers(headers: &HeaderMap) -> Self {
        trace!("Ado response headers: {:?}", headers);

        let rate_limit_resource = headers
            .get("X-RateLimit-Resource")
            .and_then(|header_value| header_value.to_str().ok())
            .unwrap_or("SSPHP:Unknown Resource")
            .to_string();
        let limits = Self {
            retry_after: Self::get_usize_from_header(headers, "Retry-After"),
            rate_limit_resource,
            rate_limit_delay: Self::get_usize_from_header(headers, "X-RateLimit-Delay"),
            rate_limit_limit: Self::get_usize_from_header(headers, "X-RateLimit-Limit"),
            rate_limit_remaining: Self::get_usize_from_header(headers, "X-RateLimit-Remaining"),
            rate_limit_reset: Self::get_usize_from_header(headers, "X-RateLimit-Reset"),
        };
        trace!("Ado parsed limits: {:?}", limits);
        if headers.contains_key("Retry-After") || headers.contains_key("retry-after") {
            unreachable!("PLEASE DEBUG RETRY HEADERS")
        }
        limits
    }
}

#[allow(unused)]
#[derive(Debug, Default)]
pub(crate) struct AdoPaging {
    #[allow(unused)]
    pub(crate) continuation_token: Option<String>,
}

impl AdoPaging {
    pub(crate) fn from_headers(headers: &HeaderMap) -> Self {
        trace!(name="Azure Dev Ops", operation="Get Paging Headers", headers=?headers);
        let continuation_token: Option<String> = headers
            .get("X-MS-ContinuationToken")
            .and_then(|value| value.to_str().ok())
            .map(str::to_string);
        Self { continuation_token }
    }

    pub(crate) fn has_more(&self) -> bool {
        self.continuation_token.is_some()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.continuation_token.is_none()
    }

    pub(crate) fn next_token(&self) -> &str {
        self.continuation_token.as_deref().unwrap_or("NOTOKEN")
    }
}

#[derive(Debug, Deserialize, Serialize, Default)]
pub(crate) struct AdoResponse {
    pub(crate) count: usize,
    pub(crate) value: Vec<Value>,
    #[serde(skip_deserializing, default, flatten)]
    pub(crate) metadata: Option<AdoMetadata>,
}

impl AddAdoResponse for AdoResponse {
    fn values(self) -> Vec<Value> {
        self.value
    }
    fn count(&self) -> usize {
        self.count
    }
}

pub(crate) trait AddAdoResponse {
    fn values(self) -> Vec<Value>;
    fn count(&self) -> usize;
}

impl ToHecEvents for AdoResponse {
    type Item = Value;

    fn source(&self) -> &str {
        self.metadata_source()
    }

    fn sourcetype(&self) -> &str {
        self.metadata_sourcetype()
    }

    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        Box::new(self.value.iter())
    }

    fn ssphp_run_key(&self) -> &str {
        SSPHP_RUN_KEY
    }

    fn to_hec_events(&self) -> Result<Vec<data_ingester_splunk::splunk::HecEvent>> {
        let (ok, err): (Vec<_>, Vec<_>) = self
            .collection()
            .map(|ado_response| {
                let mut ado_response = ado_response.clone();
                let ssphp_debug = if let Some(metadata) = &self.metadata {
                    serde_json::to_value(metadata).unwrap_or_else(|_| {
                        serde_json::to_value("Error Getting AdoMetadata")
                            .expect("Value from static str should not fail")
                    })
                } else {
                    serde_json::to_value("No AdoMetadata")
                        .expect("Value from static str should not fail")
                };

                let _ = ado_response
                    .as_object_mut()
                    .expect("ado_response should always be accessible as an Value object")
                    .insert("SSPHP_DEBUG".into(), ssphp_debug);
                data_ingester_splunk::splunk::HecEvent::new_with_ssphp_run(
                    &ado_response,
                    self.source(),
                    self.sourcetype(),
                    self.get_ssphp_run(),
                )
            })
            .partition_result();
        if !err.is_empty() {
            return Err(anyhow!(err
                .iter()
                .map(|err| format!("{:?}", err))
                .collect::<Vec<String>>()
                .join("\n")));
        }
        Ok(ok)
    }
}

impl AdoMetadataTrait for AdoResponse {
    fn set_metadata(&mut self, metadata: AdoMetadata) {
        self.metadata = Some(metadata);
    }

    fn metadata(&self) -> Option<&AdoMetadata> {
        self.metadata.as_ref()
    }
}

impl ToHecEvents for &AdoResponse {
    type Item = Value;

    fn source(&self) -> &str {
        self.metadata
            .as_ref()
            .map(|metadata| metadata.source.as_str())
            .unwrap_or("NO ADOMETADATA FOR SOURCE")
    }

    fn sourcetype(&self) -> &str {
        self.metadata
            .as_ref()
            .map(|metadata| metadata.sourcetype.as_str())
            .unwrap_or("NO ADOMETADATA FOR SOURCETYPE")
    }

    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        Box::new(self.value.iter())
    }

    fn ssphp_run_key(&self) -> &str {
        SSPHP_RUN_KEY
    }

    fn to_hec_events(&self) -> Result<Vec<data_ingester_splunk::splunk::HecEvent>> {
        let (ok, err): (Vec<_>, Vec<_>) = self
            .collection()
            .map(|ado_response| {
                let mut ado_response = ado_response.clone();
                let ssphp_debug = if let Some(metadata) = &self.metadata {
                    serde_json::to_value(metadata).unwrap_or_else(|_| {
                        serde_json::to_value("Error Getting AdoMetadata")
                            .expect("Value from static str should not fail")
                    })
                } else {
                    serde_json::to_value("No AdoMetadata")
                        .expect("Value from static str should not fail")
                };

                let _ = ado_response
                    .as_object_mut()
                    .expect("ado_response should always be accessible as an Value object")
                    .insert("SSPHP_DEBUG".into(), ssphp_debug);
                data_ingester_splunk::splunk::HecEvent::new_with_ssphp_run(
                    &ado_response,
                    self.source(),
                    self.sourcetype(),
                    self.get_ssphp_run(),
                )
            })
            .partition_result();
        if !err.is_empty() {
            return Err(anyhow!(err
                .iter()
                .map(|err| format!("{:?}", err))
                .collect::<Vec<String>>()
                .join("\n")));
        }
        Ok(ok)
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct AdoResponseSingle {
    #[serde(flatten)]
    pub(crate) value: Value,
    #[serde(default, skip)]
    metadata: Option<AdoMetadata>,
}

impl AdoMetadataTrait for AdoResponseSingle {
    fn set_metadata(&mut self, metadata: AdoMetadata) {
        self.metadata = Some(metadata);
    }

    fn metadata(&self) -> Option<&AdoMetadata> {
        self.metadata.as_ref()
    }
}

impl AddAdoResponse for AdoResponseSingle {
    fn values(self) -> Vec<Value> {
        vec![self.value]
    }
    fn count(&self) -> usize {
        1
    }
}

impl ToHecEvents for &AdoResponseSingle {
    type Item = Value;

    fn source(&self) -> &str {
        self.metadata
            .as_ref()
            .map(|metadata| metadata.source.as_str())
            .unwrap_or("NO ADOMETADATA FOR SOURCE")
    }

    fn sourcetype(&self) -> &str {
        self.metadata
            .as_ref()
            .map(|metadata| metadata.sourcetype.as_str())
            .unwrap_or("NO ADOMETADATA FOR SOURCETYPE")
    }

    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        unimplemented!()
    }

    fn ssphp_run_key(&self) -> &str {
        SSPHP_RUN_KEY
    }

    fn to_hec_events(&self) -> Result<Vec<data_ingester_splunk::splunk::HecEvent>> {
        let mut ado_response = self.value.clone();
        let ssphp_debug = if let Some(metadata) = &self.metadata {
            serde_json::to_value(metadata).unwrap_or_else(|_| {
                serde_json::to_value("Error Getting AdoMetadata")
                    .expect("Value from static str should not fail")
            })
        } else {
            serde_json::to_value("No AdoMetadata").expect("Value from static str should not fail")
        };

        let _ = ado_response
            .as_object_mut()
            .expect("ado_response should always be accessible as an Value object")
            .insert("SSPHP_DEBUG".into(), ssphp_debug);
        Ok(vec![
            data_ingester_splunk::splunk::HecEvent::new_with_ssphp_run(
                &ado_response,
                self.source(),
                self.sourcetype(),
                self.get_ssphp_run(),
            )?,
        ])
    }
}
