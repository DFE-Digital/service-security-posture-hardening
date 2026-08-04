use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvironmentResponse {
    pub id: String,
    pub display_name: String,
    #[serde(default)]
    pub state: Option<String>,
    #[serde(default)]
    pub r#type: Option<String>,
    #[serde(default)]
    pub tenant_id: Option<String>,
    #[serde(default)]
    pub geo: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
}
