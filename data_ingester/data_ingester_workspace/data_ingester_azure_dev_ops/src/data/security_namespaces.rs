use serde::Deserialize;
use serde::Serialize;
use tracing::error;

use crate::ado_response::AdoResponse;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SecurityNamespaces {
    pub namespaces: Vec<SecurityNamespace>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SecurityNamespace {
    // pub collection: Collection,
    // pub default_team: DefaultTeam,
    pub namespace_id: String,
    pub name: String,
    // pub last_update_time: String,
    // pub name: String,
    // pub revision: i64,
    // pub state: String,
    // pub url: String,
    // pub visibility: String,
}

impl From<AdoResponse> for SecurityNamespaces {
    fn from(value: AdoResponse) -> Self {
        let collection = value.value.into_iter().filter_map(|repo| {
            match serde_json::from_value::<SecurityNamespace>(repo) {
                Ok(repo) => {
                    Some(repo)
                },
                Err(err) => {
                    error!(name="Azure DevOps", operation="From<AdoResponse> for Repositories", error=?err);
                    None
                }
            }
        }).collect();
        Self {
            namespaces: collection,
        }
    }
}
