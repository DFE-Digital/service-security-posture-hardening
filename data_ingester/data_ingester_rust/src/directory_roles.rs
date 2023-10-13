// use crate::splunk::ToHecEvents;
use serde::Deserialize;
use serde::Serialize;
use serde_with::skip_serializing_none;

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
    pub(crate) role_template_id: String,
    pub(crate) members: Option<Vec<DirectoryRoleMember>>,
    pub(crate) is_privileged: Option<bool>,
}

#[derive(Debug, Serialize, Default)]
pub struct DirectoryRoles<'a> {
    pub value: Vec<&'a DirectoryRole>,
}

// #[derive(Debug, Serialize, Deserialize, Default)]
// pub struct DirectoryRoles<'a> {
//     pub value: Vec<Cow<'a, DirectoryRole>>,
// }

// #[derive(Debug, Serialize, Deserialize)]
// pub struct DirectoryRoles {
//     pub value: Vec<&'a DirectoryRole>,
// }

impl DirectoryRoles<'_> {
    // pub fn new() -> Self {
    //     Self { value: Vec::new() }
    // }

    pub fn ids(&self) -> Vec<&'_ str> {
        self.value.iter().map(|role| role.id.as_str()).collect()
    }
}

// impl<'a> ToHecEvents for DirectoryRoles<'a> {
//     fn source() -> &'static str {
//         "msgraph"
//     }

//     fn sourcetype() -> &'static str {
//         "msgraph:directory_role"
//     }
// }

// impl<'a> IntoIterator for DirectoryRoles<'a> {
//     type Item = DirectoryRole;
//     type IntoIter = std::vec::IntoIter<Self::Item>;

//     fn into_iter(self) -> Self::IntoIter {
//         self.value.into_iter()
//     }
// }

#[test]
fn directory_role_ids() {
    let role1 = DirectoryRole {
        id: "id_1".to_owned(),
        display_name: None,
        role_template_id: "role1id".to_owned(),
        members: None,
        is_privileged: None,
    };
    let role2 = DirectoryRole {
        id: "id_2".to_owned(),
        display_name: None,
        role_template_id: "role2id".to_owned(),
        members: None,
        is_privileged: None,
    };
    let roles = vec![role1, role2];
    let roles = roles.iter().collect::<DirectoryRoles>();
    assert_eq!(roles.ids(), ["id_1", "id_2"]);
}

impl<'a> FromIterator<&'a DirectoryRole> for DirectoryRoles<'a> {
    fn from_iter<I: IntoIterator<Item = &'a DirectoryRole>>(iter: I) -> Self {
        let mut value = vec![];
        for i in iter {
            value.push(i);
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

//pub struct DirectoryRoleTemplate {}
// impl DirectoryRoleTemplates {
//     pub fn new() -> Self {
//         Self { value: Vec::new() }
//     }
// }

// impl ToHecEvents for DirectoryRoleTemplates {
//     fn source() -> &'static str {
//         "msgraph"
//     }

//     fn sourcetype() -> &'static str {
//         "msgraph:dircetory_role_templates"
//     }
// }

// impl IntoIterator for &DirectoryRoleTemplates {
//     type Item = Cow<'_, DirectoryRoleTemplate>;
//     type IntoIter = std::vec::Iter<Self::Item>;

//     fn into_iter(self) -> Self::IntoIter {
//         self.value.iter()
//     }
// }
