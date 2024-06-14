use std::iter;

use data_ingester_splunk::splunk::ToHecEvents;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SecurityScores {
    #[serde(rename = "@odata.context")]
    pub odata_context: String,
    #[serde(rename = "@odata.nextLink")]
    pub odata_next_link: Option<String>,
    #[serde(rename = "value")]
    pub inner: Vec<SecurityScore>,
}

impl ToHecEvents for &SecurityScores {
    type Item = Value;
    fn source(&self) -> &str {
        "msgraph"
    }

    fn sourcetype(&self) -> &str {
        "m365:control_score"
    }

    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        match self.inner.first() {
            Some(first) => Box::new(first.control_scores.iter()),
            None => Box::new(iter::empty()),
        }
    }
    fn ssphp_run_key(&self) -> &str {
        "m365"
    }
}

#[cfg(test)]
mod test {
    use crate::ms_graph::test::setup;

    use data_ingester_splunk::splunk::ToHecEvents;

    use anyhow::Result;
    #[tokio::test]
    async fn test_to_hec_events_collection() -> Result<()> {
        let (_splunk, ms_graph) = setup().await?;
        let security_scores = ms_graph.get_security_secure_scores().await?;
        let _hec_events = (&security_scores).to_hec_events()?;
        Ok(())
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
    #[serde(skip_serializing)]
    pub control_scores: Vec<Value>,
}

//impl ToHecEvents for &SecurityScore {
//     type Item = Self;
//     fn source(&self) -> &str {
//         "msgraph"
//     }

//     fn sourcetype(&self) -> &str {
//         "m365:security_score"
//     }

//     fn to_hec_events(&self) -> anyhow::Result<Vec<crate::splunk::HecEvent>> {
//         Ok(vec![HecEvent::new(
//             &self,
//             self.source(),
//             self.sourcetype(),
//         )?])
//     }

//     fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
//         unimplemented!()
//     }
// }

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

// #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
// #[serde(rename_all = "camelCase")]
// pub struct ControlScore {
//     pub control_category: String,
//     pub control_name: String,
//     pub description: String,
//     pub score: f64,
//     pub source: Option<String>,
//     #[serde(rename = "IsApplicable")]
//     pub is_applicable: Option<String>,
//     pub score_in_percentage: Option<f64>,
//     pub on: Option<String>,
//     pub last_synced: String,
//     pub implementation_status: String,
//     pub count: Option<String>,
//     pub total: Option<String>,
// }
