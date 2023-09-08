use crate::splunk::HecEvent;
use serde::Deserialize;
use serde::Serialize;
use serde_with::skip_serializing_none;
use std::borrow::Cow;

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DirectoryRoleMember {
    pub(crate) id: Option<String>,
    // "deletedDateTime": Null,
    // "description": String("Read the definition of custom security attributes."),
    display_name: Option<String>,
}

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct DirectoryRole {
    pub(crate) id: String,
    // "deletedDateTime": Null,
    // "description": String("Read the definition of custom security attributes."),
    pub(crate) display_name: Option<String>,
    pub(crate) role_template_id: Option<String>,
    pub(crate) members: Option<Vec<DirectoryRoleMember>>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct DirectoryRoles<'a> {
    pub value: Vec<Cow<'a, DirectoryRole>>,
}

// #[derive(Debug, Serialize, Deserialize)]
// pub struct DirectoryRoles {
//     pub value: Vec<&'a DirectoryRole>,
// }

impl DirectoryRoles<'_> {
    pub fn new() -> Self {
        Self { value: Vec::new() }
    }

    pub fn to_hec_event(&self) -> Vec<HecEvent> {
        self.value
            .iter()
            .map(|u| HecEvent::new(u, "msgraph", "msgraph:directory_roles"))
            .collect()
    }

    pub fn ids(&self) -> Vec<&'_ str> {
        self.value.iter().map(|role| role.id.as_str()).collect()
    }
}

#[test]
fn directory_role_ids() {
    let role1 = DirectoryRole {
        id: "id_1".to_owned(),
        display_name: None,
        role_template_id: None,
        members: None,
    };
    let role2 = DirectoryRole {
        id: "id_2".to_owned(),
        display_name: None,
        role_template_id: None,
        members: None,
    };
    let roles = vec![role1, role2];
    let roles = roles.iter().collect::<DirectoryRoles>();
    assert_eq!(roles.ids(), ["id_1", "id_2"]);
}

impl<'a> FromIterator<&'a DirectoryRole> for DirectoryRoles<'a> {
    fn from_iter<I: IntoIterator<Item = &'a DirectoryRole>>(iter: I) -> Self {
        let mut value = vec![];
        for i in iter {
            value.push(Cow::Borrowed(i));
        }
        Self { value }
    }
}

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DirectoryRoleTemplates {
    pub value: Vec<serde_json::Value>,
}

pub struct DirectoryRoleTemplate {}
impl DirectoryRoleTemplates {
    pub fn new() -> Self {
        Self { value: Vec::new() }
    }

    pub fn to_hec_event(&self) -> Vec<HecEvent> {
        self.value
            .iter()
            .map(|u| HecEvent::new(u, "msgraph", "msgraph:directory_roles"))
            .collect()
    }
}
