use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;

use crate::splunk::ToHecEvent;

impl ToHecEvent for AdminRequestConsentPolicy {
    fn source() -> &'static str {
        "msgraph"
    }

    fn sourcetype() -> &'static str {
        "admin_request_consent_policy"
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminRequestConsentPolicy {
    #[serde(rename = "@odata.context")]
    pub odata_context: String,
    pub is_enabled: bool,
    pub notify_reviewers: bool,
    pub reminders_enabled: bool,
    pub request_duration_in_days: i64,
    pub version: i64,
    pub reviewers: Vec<AdminRequestConsentPolicyReviewer>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminRequestConsentPolicyReviewer {
    pub query: String,
    pub query_type: String,
    pub query_root: Value,
}
