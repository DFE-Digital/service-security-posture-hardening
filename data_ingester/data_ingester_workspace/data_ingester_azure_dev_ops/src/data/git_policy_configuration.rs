use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;
use tracing::error;

use crate::ado_response::AdoResponse;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PolicyConfigurations {
    pub policies: Vec<PolicyConfiguration>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PolicyConfiguration {
    #[serde(rename = "_links")]
    pub links: Links,
    pub created_by: CreatedBy,
    pub created_date: String,
    pub id: i64,
    pub is_blocking: bool,
    pub is_deleted: bool,
    pub is_enabled: bool,
    pub is_enterprise_managed: bool,
    pub revision: i64,
    pub settings: Settings,
    #[serde(rename = "type")]
    pub type_field: Type,
    pub url: String,
    #[serde(skip)]
    pub project_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Links {
    pub policy_type: PolicyType,
    #[serde(rename = "self")]
    pub self_field: SelfField,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PolicyType {
    pub href: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SelfField {
    pub href: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatedBy {
    #[serde(rename = "_links")]
    pub links: Links2,
    pub descriptor: String,
    pub display_name: String,
    pub id: String,
    pub image_url: String,
    pub unique_name: String,
    pub url: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Links2 {
    pub avatar: Avatar,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Avatar {
    pub href: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub scope: Vec<Scope>,
    //#[serde(flatten)]
    // pub settings_type: SettingsType,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SettingsType {
    Build(SettingsBuild),
    Unknown(Value),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsBuild {
    pub build_definition_id: u32,
    pub display_name: Option<String>,
    pub manual_queue_only: bool,
    pub queue_on_source_update_only: bool,
    pub filename_patterns: Option<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Scope {
    pub repository_id: Option<String>,
    pub match_kind: Option<MatchKind>,
    pub ref_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MatchKind {
    Exact,
    Prefix,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Type {
    pub display_name: String,
    pub id: String,
    pub url: String,
}

impl From<(AdoResponse, &str)> for PolicyConfigurations {
    fn from(value: (AdoResponse, &str)) -> Self {
        let (value, project_id) = value;
        let policies = value.value.into_iter().filter_map(|policy| {
            match serde_json::from_value::<PolicyConfiguration>(policy) {
                Ok(mut policy) => {
                    policy.project_id = Some(project_id.to_string());
                    Some(policy)
                },
                Err(err) => {
                    error!(name="Azure DevOps", operation="From<AdoResponse> for PolicyConfiguration", error=?err);
                    None
                }
            }
        }).collect();
        Self { policies }
    }
}
