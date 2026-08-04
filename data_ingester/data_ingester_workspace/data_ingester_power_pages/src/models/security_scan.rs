use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
#[allow(dead_code)]
pub struct SiteSecurityScore {
    pub succeeded_rules: i32,
    pub total_rules: i32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct SiteSecurityResult {
    #[serde(default)]
    pub start_time: Option<String>,
    #[serde(default)]
    pub end_time: Option<String>,
    #[serde(default)]
    pub failed_rule_count: Option<i32>,
    #[serde(default)]
    pub total_alert_count: Option<i32>,
    #[serde(default)]
    pub total_rule_count: Option<i32>,
    #[serde(default)]
    pub user_name: Option<String>,
    #[serde(default)]
    pub rules: Vec<ScanRule>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct ScanRule {
    #[serde(default)]
    pub rule_id: Option<String>,
    #[serde(default)]
    pub rule_name: Option<String>,
    #[serde(default)]
    pub rule_status: Option<String>,
    #[serde(default)]
    pub alerts_count: Option<i32>,
    #[serde(default)]
    pub alerts: Vec<serde_json::Value>,
}
