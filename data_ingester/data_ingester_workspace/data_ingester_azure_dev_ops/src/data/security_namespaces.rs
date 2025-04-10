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

impl SecurityNamespace {}

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

#[cfg(test)]
pub(crate) mod test {
    use crate::{
        ado_dev_ops_client::AzureDevOpsClientMethods, ado_response::AdoResponse,
        test_utils::TEST_SETUP,
    };

    use super::SecurityNamespaces;

    fn security_namespace_ado_response() -> AdoResponse {
        let t = &*TEST_SETUP;
        let ado_response: AdoResponse = t
            .runtime
            .block_on(async { t.ado.security_namespaces(&t.organization).await.unwrap() });
        ado_response
    }

    pub(crate) fn security_namespace() -> SecurityNamespaces {
        let ado_response = security_namespace_ado_response();
        SecurityNamespaces::from(ado_response)
    }

    #[test]
    fn security_namespaces_from_ado_response() {
        let security_namespaces = security_namespace();
        assert_eq!(security_namespaces.namespaces.len(), 61);
    }
}
