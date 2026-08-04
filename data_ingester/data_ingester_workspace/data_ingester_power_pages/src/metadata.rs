use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DataType {
    Environment,
    Website,
    Hostname,
    SslBinding,
    DeepScanReport,
    DeepScanScore,
    WafStatus,
    WafCustomRule,
    WafManagedRuleSet,
    WafManagedRuleOverride,
    WafInactive,
    AllowedIp,
    Certificate,
    NoPowerPagesSites,
    Preflight,
    RunSummary,
    Diagnostic,
}

#[derive(Debug, Clone, Serialize)]
pub struct PowerPagesMetadata {
    pub data_type: DataType,
    pub environment_id: Option<String>,
    pub environment_name: Option<String>,
    pub website_id: Option<String>,
    pub website_name: Option<String>,
    pub website_url: Option<String>,
    pub request_url: String,
    pub hostname: Option<String>,
    pub cert_type: Option<String>,
    pub is_deep_scan: bool,
}

impl PowerPagesMetadata {
    pub fn source(&self) -> String {
        match self.data_type {
            DataType::Environment | DataType::NoPowerPagesSites => format!(
                "power_pages:environment:{}",
                self.environment_id.as_deref().unwrap_or("unknown")
            ),
            DataType::Website => format!(
                "power_pages:websites:{}",
                self.environment_id.as_deref().unwrap_or("unknown")
            ),
            DataType::DeepScanReport => format!(
                "power_pages:deep_scan:{}",
                self.website_id.as_deref().unwrap_or("unknown")
            ),
            DataType::DeepScanScore => format!(
                "power_pages:deep_scan_score:{}",
                self.website_id.as_deref().unwrap_or("unknown")
            ),
            DataType::WafCustomRule => format!(
                "power_pages:waf_custom_rule:{}",
                self.website_id.as_deref().unwrap_or("unknown")
            ),
            DataType::WafManagedRuleSet => format!(
                "power_pages:waf_managed_rule_set:{}",
                self.website_id.as_deref().unwrap_or("unknown")
            ),
            DataType::WafManagedRuleOverride => format!(
                "power_pages:waf_managed_rule:{}",
                self.website_id.as_deref().unwrap_or("unknown")
            ),
            DataType::WafStatus | DataType::WafInactive => format!(
                "power_pages:waf_status:{}",
                self.website_id.as_deref().unwrap_or("unknown")
            ),
            DataType::AllowedIp => format!(
                "power_pages:allowed_ips:{}",
                self.website_id.as_deref().unwrap_or("unknown")
            ),
            DataType::Certificate => format!(
                "power_pages:certificates:{}",
                self.website_id.as_deref().unwrap_or("unknown")
            ),
            DataType::Hostname => format!(
                "power_pages:hostnames:{}",
                self.website_id.as_deref().unwrap_or("unknown")
            ),
            DataType::SslBinding => format!(
                "power_pages:ssl_bindings:{}",
                self.website_id.as_deref().unwrap_or("unknown")
            ),
            DataType::Preflight => "power_pages:preflight".to_string(),
            DataType::RunSummary => "power_pages:run_summary".to_string(),
            DataType::Diagnostic => self
                .website_id
                .as_ref()
                .map(|id| format!("power_pages:diagnostic:{id}"))
                .unwrap_or_else(|| "power_pages:diagnostic".to_string()),
        }
    }

    pub fn for_environment(env: &crate::models::EnvironmentResponse) -> Self {
        Self {
            data_type: DataType::Environment,
            environment_id: Some(env.id.clone()),
            environment_name: Some(env.display_name.clone()),
            website_id: None,
            website_name: None,
            website_url: None,
            request_url: String::new(),
            hostname: None,
            cert_type: None,
            is_deep_scan: false,
        }
    }

    pub fn for_website(
        env: &crate::models::EnvironmentResponse,
        website: &crate::models::WebsiteDto,
        data_type: DataType,
        request_url: impl Into<String>,
    ) -> Self {
        Self {
            data_type,
            environment_id: Some(env.id.clone()),
            environment_name: Some(env.display_name.clone()),
            website_id: Some(website.id.clone()),
            website_name: Some(website.name.clone()),
            website_url: Some(website.website_url.clone()),
            request_url: request_url.into(),
            hostname: None,
            cert_type: None,
            is_deep_scan: matches!(
                data_type,
                DataType::DeepScanReport | DataType::DeepScanScore
            ),
        }
    }
}
