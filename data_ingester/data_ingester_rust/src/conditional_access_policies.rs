use std::collections::HashSet;

use crate::splunk::ToHecEvents;
//use crate::splunk::ToHecEvents;
use crate::users::User;
use serde::Deserialize;
use serde::Serialize;
use serde_with::skip_serializing_none;
#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ConditionalAccessPolicy {
    pub(crate) id: String,
    display_name: Option<String>,
    state: Option<String>,
    conditions: ConditionalAccessPolicyConditions,
    grant_controls: serde_json::Value,
}

impl ConditionalAccessPolicy {
    pub fn affects_user(&self, user: &User) -> bool {
        let condition_users = &self.conditions.users;
        let result = {
            // Users
            if condition_users.include_users.contains(&"All".to_owned()) {
                return true;
            }
            if condition_users.include_users.contains(&user.id) {
                return true;
            }

            // Groups
            if condition_users.include_groups.contains(&"All".to_owned()) {
                return true;
            }
            for group_id in user.groups().ids() {
                if condition_users
                    .include_groups
                    .contains(&group_id.to_owned())
                {
                    return true;
                }
            }

            // Roles
            if condition_users.include_roles.contains(&"All".to_owned()) {
                return true;
            }
            for role_id in user.roles().ids() {
                if condition_users.include_roles.contains(&role_id.to_owned()) {
                    return true;
                }
            }
            false
        };

        // Cap dosen't apply
        if !result {
            return result;
        }

        // Check excludes
        {
            // Users
            if condition_users.exclude_users.contains(&"All".to_owned()) {
                return false;
            }
            if condition_users.exclude_users.contains(&user.id) {
                return false;
            }

            // Groups
            if condition_users.exclude_groups.contains(&"All".to_owned()) {
                return false;
            }
            for group_id in user.groups().ids() {
                if condition_users
                    .exclude_groups
                    .contains(&group_id.to_owned())
                {
                    return false;
                }
            }

            // Roles
            if condition_users.exclude_roles.contains(&"All".to_owned()) {
                return false;
            }
            for role_id in user.roles().ids() {
                if condition_users.exclude_roles.contains(&role_id.to_owned()) {
                    return false;
                }
            }
            true
        }
    }

    pub fn to_user_conditional_access_policy(&self) -> UserConditionalAccessPolicy {
        UserConditionalAccessPolicy {
            id: self.id.as_str(),
            display_name: self.display_name.as_deref(),
            state: self.state.as_deref(),
        }
    }
}

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ConditionalAccessPolicyConditions {
    users: ConditionalAccessPolicyConditionsUsers,
}

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ConditionalAccessPolicyConditionsUsers {
    exclude_groups: HashSet<String>,
    exclude_guests_or_external_users: Option<serde_json::Value>,
    exclude_roles: HashSet<String>,
    exclude_users: HashSet<String>,
    include_groups: HashSet<String>,
    include_guests_or_external_users: Option<serde_json::Value>,
    include_roles: HashSet<String>,
    include_users: HashSet<String>,
}

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ConditionalAccessPolicies {
    pub value: Vec<ConditionalAccessPolicy>,
}

impl<'a> ToHecEvents<'a> for ConditionalAccessPolicies {
    type Item = ConditionalAccessPolicy;
    fn source() -> &'static str {
        "msgraph"
    }

    fn sourcetype() -> &'static str {
        "SSPHP.AAD.conditional_access_policy"
    }
    fn collection(&'a self) -> Box<dyn Iterator<Item = &Self::Item> + 'a> {
        Box::new(self.value.iter())
    }
}

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct UserConditionalAccessPolicy<'a> {
    id: &'a str,
    display_name: Option<&'a str>,
    state: Option<&'a str>,
}

impl<'a> UserConditionalAccessPolicy<'a> {}

impl ConditionalAccessPolicies {
    pub fn new() -> Self {
        Self { value: Vec::new() }
    }
}

impl IntoIterator for ConditionalAccessPolicies {
    type Item = ConditionalAccessPolicy;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.value.into_iter()
    }
}

#[cfg(test)]
mod conditional_access_policy {
    use super::ConditionalAccessPolicy;
    use crate::{directory_roles::DirectoryRole, groups::Group, users::GroupOrRole, users::User};

    fn setup() -> (User<'static>, ConditionalAccessPolicy) {
        let mut user = User::new("123".to_owned(), "test_user".to_owned());
        user.transitive_member_of = Some(vec![
            GroupOrRole::Group(Group {
                id: "group1".to_owned(),
                display_name: Some("group_1_name".to_owned()),
            }),
            GroupOrRole::Role(DirectoryRole {
                id: "role1".to_owned(),
                display_name: Some("role_1_name".to_owned()),
                role_template_id: "role1id".to_owned(),
                members: None,
                is_privileged: None,
            }),
        ]);
        let cap = ConditionalAccessPolicy::default();
        (user, cap)
    }
    #[test]
    fn affected_user_not_in_cap() {
        let (user, cap) = setup();
        assert!(!cap.affects_user(&user));
    }

    #[test]
    fn affected_user_by_user_id() {
        let (user, mut cap) = setup();
        cap.conditions
            .users
            .include_users
            .insert(user.id.to_owned());
        assert!(cap.affects_user(&user));
    }

    #[test]
    fn affected_user_by_user_all() {
        let (user, mut cap) = setup();
        cap.conditions.users.include_users.insert("All".to_owned());
        assert!(cap.affects_user(&user));
    }

    #[test]
    fn affected_user_by_group_all() {
        let (user, mut cap) = setup();
        cap.conditions.users.include_groups.insert("All".to_owned());
        assert!(cap.affects_user(&user));
    }
    #[test]
    fn affected_user_by_group_id() {
        let (user, mut cap) = setup();
        cap.conditions
            .users
            .include_groups
            .insert("group1".to_owned());
        assert!(cap.affects_user(&user));
    }
    #[test]
    fn affected_user_by_role_all() {
        let (user, mut cap) = setup();
        cap.conditions.users.include_roles.insert("All".to_owned());
        assert!(cap.affects_user(&user));
    }
    #[test]
    fn affected_user_by_role_id() {
        let (user, mut cap) = setup();
        cap.conditions
            .users
            .include_roles
            .insert("role1".to_owned());
        assert!(cap.affects_user(&user));
    }

    #[test]
    fn affected_user_excluded_by_user_id() {
        let (user, mut cap) = setup();
        cap.conditions
            .users
            .include_users
            .insert(user.id.to_owned());
        cap.conditions
            .users
            .exclude_users
            .insert(user.id.to_owned());
        assert!(cap.affects_user(&user));
    }

    #[test]
    fn affected_user_excluded_by_user_all() {
        let (user, mut cap) = setup();
        cap.conditions
            .users
            .include_users
            .insert(user.id.to_owned());
        cap.conditions.users.exclude_users.insert("All".to_owned());
        assert!(cap.affects_user(&user));
    }

    #[test]
    fn affected_user_excluded_by_group_all() {
        let (user, mut cap) = setup();
        cap.conditions
            .users
            .include_users
            .insert(user.id.to_owned());
        cap.conditions.users.exclude_groups.insert("All".to_owned());
        assert!(cap.affects_user(&user));
    }
    #[test]
    fn affected_user_excluded_by_group_id() {
        let (user, mut cap) = setup();
        cap.conditions
            .users
            .include_users
            .insert(user.id.to_owned());
        cap.conditions
            .users
            .exclude_groups
            .insert("group1".to_owned());
        assert!(cap.affects_user(&user));
    }
    #[test]
    fn affected_user_excluded_by_role_all() {
        let (user, mut cap) = setup();
        cap.conditions
            .users
            .include_users
            .insert(user.id.to_owned());
        cap.conditions.users.exclude_roles.insert("All".to_owned());
        assert!(cap.affects_user(&user));
    }
    #[test]
    fn affected_user_excluded_by_role_id() {
        let (user, mut cap) = setup();
        cap.conditions
            .users
            .include_users
            .insert(user.id.to_owned());
        cap.conditions
            .users
            .exclude_roles
            .insert("role1".to_owned());
        assert!(cap.affects_user(&user));
    }
}
