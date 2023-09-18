use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::splunk::ToHecEvent;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SecurityScores {
    #[serde(rename = "@odata.context")]
    pub odata_context: String,
    #[serde(rename = "@odata.nextLink")]
    pub odata_next_link: Option<String>,
    pub value: Vec<SecurityScore>,
}

impl ToHecEvent for SecurityScores {
    fn source() -> &'static str {
        "msgraph"
    }

    fn sourcetype() -> &'static str {
        "m365"
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SecurityScore {
    #[serde(rename = "@odata.context")]
    pub odata_context: Option<String>,
    pub id: String,
    pub azure_tenant_id: String,
    pub active_user_count: i64,
    pub created_date_time: String,
    pub current_score: f64,
    pub enabled_services: Vec<String>,
    pub licensed_user_count: i64,
    pub max_score: f64,
    pub vendor_information: VendorInformation,
    pub average_comparative_scores: Vec<AverageComparativeScore>,
    pub control_scores: Vec<ControlScore>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VendorInformation {
    pub provider: String,
    pub provider_version: Value,
    pub sub_provider: Value,
    pub vendor: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AverageComparativeScore {
    pub basis: String,
    pub average_score: f64,
    pub apps_score: f64,
    pub apps_score_max: f64,
    pub data_score: f64,
    pub data_score_max: f64,
    pub device_score: f64,
    pub device_score_max: f64,
    pub identity_score: f64,
    pub identity_score_max: f64,
    pub infrastructure_score: f64,
    pub infrastructure_score_max: f64,
    #[serde(rename = "SeatSizeRangeLowerValue")]
    pub seat_size_range_lower_value: Option<String>,
    #[serde(rename = "SeatSizeRangeUpperValue")]
    pub seat_size_range_upper_value: Option<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ControlScore {
    pub control_category: String,
    pub control_name: String,
    pub description: String,
    pub score: f64,
    pub source: String,
    #[serde(rename = "IsApplicable")]
    pub is_applicable: String,
    pub score_in_percentage: f64,
    pub on: Option<String>,
    pub last_synced: String,
    pub implementation_status: String,
    pub count: Option<String>,
    pub total: Option<String>,
}

impl ToHecEvent for SecurityScore {
    fn source() -> &'static str {
        "msgraph"
    }

    fn sourcetype() -> &'static str {
        "m365"
    }
}
