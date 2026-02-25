use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;
use tracing::error;

use crate::ado_response::AdoResponse;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Identities {
    pub(crate) inner: Vec<Identity>,
}

impl From<AdoResponse> for Identities {
    fn from(value: AdoResponse) -> Self {
        let identities = value
            .value
            .into_iter()
            .filter(|identity_value| !identity_value.is_null())
            .filter_map(|identity_value| {
                if let Ok(identity) = serde_json::from_value(identity_value) {
                    Some(identity)
                } else {
                    error!("Failed to build Identity from Value");
                    None
                }
            })
            .collect::<Vec<Identity>>();
        Self { inner: identities }
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Identity {
    pub descriptor: String,
    pub id: String,
    pub is_active: Option<bool>,
    pub is_container: Option<bool>,
    pub member_ids: Option<Vec<Value>>,
    pub member_of: Option<Vec<Value>>,
    pub members: Option<Vec<Value>>,
    pub meta_type_id: i64,
    pub properties: Properties,
    pub provider_display_name: String,
    pub resource_version: i64,
    pub subject_descriptor: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Properties {
    #[serde(rename = "Account")]
    pub account: Account,
    #[serde(rename = "Description")]
    pub description: Description,
    #[serde(rename = "Domain")]
    pub domain: Domain,
    #[serde(rename = "GlobalScope")]
    pub global_scope: Option<GlobalScope>,
    #[serde(rename = "LocalScopeId")]
    pub local_scope_id: Option<LocalScopeId>,
    #[serde(rename = "SchemaClassName")]
    pub schema_class_name: SchemaClassName,
    #[serde(rename = "ScopeId")]
    pub scope_id: Option<ScopeId>,
    #[serde(rename = "ScopeName")]
    pub scope_name: Option<ScopeName>,
    #[serde(rename = "ScopeType")]
    pub scope_type: Option<ScopeType>,
    #[serde(rename = "SecuringHostId")]
    pub securing_host_id: Option<SecuringHostId>,
    #[serde(rename = "SecurityGroup")]
    pub security_group: Option<SecurityGroup>,
    #[serde(rename = "SpecialType")]
    pub special_type: SpecialType,
    #[serde(rename = "VirtualPlugin")]
    pub virtual_plugin: Option<VirtualPlugin>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    #[serde(rename = "$type")]
    pub type_field: String,
    #[serde(rename = "$value")]
    pub value: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Description {
    #[serde(rename = "$type")]
    pub type_field: String,
    #[serde(rename = "$value")]
    pub value: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Domain {
    #[serde(rename = "$type")]
    pub type_field: String,
    #[serde(rename = "$value")]
    pub value: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GlobalScope {
    #[serde(rename = "$type")]
    pub type_field: String,
    #[serde(rename = "$value")]
    pub value: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalScopeId {
    #[serde(rename = "$type")]
    pub type_field: String,
    #[serde(rename = "$value")]
    pub value: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SchemaClassName {
    #[serde(rename = "$type")]
    pub type_field: String,
    #[serde(rename = "$value")]
    pub value: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScopeId {
    #[serde(rename = "$type")]
    pub type_field: String,
    #[serde(rename = "$value")]
    pub value: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScopeName {
    #[serde(rename = "$type")]
    pub type_field: String,
    #[serde(rename = "$value")]
    pub value: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScopeType {
    #[serde(rename = "$type")]
    pub type_field: String,
    #[serde(rename = "$value")]
    pub value: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SecuringHostId {
    #[serde(rename = "$type")]
    pub type_field: String,
    #[serde(rename = "$value")]
    pub value: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SecurityGroup {
    #[serde(rename = "$type")]
    pub type_field: String,
    #[serde(rename = "$value")]
    pub value: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpecialType {
    #[serde(rename = "$type")]
    pub type_field: String,
    #[serde(rename = "$value")]
    pub value: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VirtualPlugin {
    #[serde(rename = "$type")]
    pub type_field: String,
    #[serde(rename = "$value")]
    pub value: String,
}

#[cfg(test)]
pub(crate) mod test {
    use crate::{
        ado_dev_ops_client::AzureDevOpsClientMethods, ado_response::AdoResponse,
        data::security_acl::test::acls_from_ado_response, test_utils::TEST_SETUP,
    };
    use anyhow::Result;

    use super::Identities;

    
    pub(crate) fn identities_ado_response() -> AdoResponse {
        let t = &*TEST_SETUP;
        let acls = acls_from_ado_response();
        let descriptors = acls.all_acl_descriptors();
        let descriptor = descriptors
            .iter()
            .find(|descriptor| descriptor.contains("2179408616-0-0-0-0-1"))
            .unwrap();
        let result: Result<AdoResponse> = t.runtime.block_on(async {
            let result = t.ado.identities(&t.organization, descriptor).await?;
            assert!(!result.value.is_empty());
            Ok(result)
        });
        result.unwrap()
    }

    
    #[test]
    #[ignore]
    fn test_identities_from_ado_response() {
        let ado_response = identities_ado_response();
        let identities = Identities::from(ado_response);
        assert!(!identities.inner.is_empty());
    }
}
