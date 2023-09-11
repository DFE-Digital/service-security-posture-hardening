use std::collections::HashMap;

use crate::splunk::ToHecEvents;
use serde::Deserialize;
use serde::Deserializer;
use serde::Serialize;
use serde_with::skip_serializing_none;

// https://learn.microsoft.com/en-us/graph/api/resources/user?view=graph-rest-1.0
#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RoleDefinition {
    pub(crate) id: String,
    description: Option<String>,
    display_name: Option<String>,
    // "inheritsPermissionsFrom": Option<Vec,
    // "inheritsPermissionsFrom@odata.context": Option<String>,
    is_built_in: Option<bool>,
    is_enabled: Option<bool>,
    pub is_privileged: Option<bool>,
    // "resourceScopes": Array [
    //     String("/"),
    // ],
    // "rolePermissions": Array [
    //     Object {
    //         "allowedResourceActions": Array [
    //             String("microsoft.directory/applicationPolicies/allProperties/read"),
    //         ],
    //         "condition": Null,
    //     },
    // ],
    // template_id: Option<String>,
    // version: Null,
}

trait Indexable {
    fn get_id(&self) -> String;
}

impl Indexable for RoleDefinition {
    fn get_id(&self) -> String {
        self.id.to_owned()
    }
}
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct RoleDefinitions {
    //    pub value: Vec<RoleDefinition>,
    #[serde(deserialize_with = "index_by_id")]
    pub value: HashMap<String, RoleDefinition>,
}

fn index_by_id<'de, D, T: ?Sized + Indexable>(d: D) -> Result<HashMap<String, T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    let data = <Vec<T>>::deserialize(d)?;
    let mapped = data.into_iter().map(|elem| (elem.get_id(), elem)).collect();
    Ok(mapped)
}

impl RoleDefinitions {
    pub fn new() -> Self {
        Self {
            value: HashMap::new(),
        }
    }
}

// impl ToHecEvents for RoleDefinitions {
//     fn source() -> &'static str {
//         "msgraph"
//     }

//     fn sourcetype() -> &'static str {
//         "msgraph:role_definition"
//     }
// }

// impl IntoIterator for &RoleDefinitions {
//     type Item = RoleDefinition;
//     type IntoIter = std::collections::Hash_map::Iter<'_, std::string::String, RoleDefinition>;

//     fn into_iter(self) -> Self::IntoIter {
//         self.value
//             .iter()
//     }
// }
