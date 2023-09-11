use crate::conditional_access_policies::ConditionalAccessPolicies;
use crate::conditional_access_policies::UserConditionalAccessPolicy;
use crate::directory_roles::DirectoryRole;
use crate::directory_roles::DirectoryRoles;
use crate::groups::Group;
use crate::groups::Groups;
use crate::roles::RoleDefinitions;
use crate::splunk::ToHecEventss;
use serde::Deserialize;
use serde::Serialize;
use serde_with::skip_serializing_none;

// https://learn.microsoft.com/en-us/graph/api/resources/user?view=graph-rest-1.0
#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct User<'a> {
    pub(crate) id: String,
    assigned_plans: Option<Vec<AssignedPlan>>,
    // business_phones: Option<Vec<String>>,
    pub(crate) display_name: Option<String>,
    given_name: Option<String>,
    //job_title: Option<String>,
    mail: Option<String>,
    //mobile_phone: Option<String>,
    //office_location: Option<String>,
    //preferred_language: Option<String>,
    surname: Option<String>,
    user_principal_name: Option<String>,
    // Requires scope: AuditLog.Read.All
    sign_in_activity: Option<String>,
    account_enabled: Option<bool>,
    pub(crate) transitive_member_of: Option<Vec<GroupOrRole>>,
    #[serde(skip_deserializing)]
    conditional_access_policies: Option<Vec<UserConditionalAccessPolicy<'a>>>,
    // TODO!
    is_privileged: Option<bool>,
    // TODO!
    // assigned_plans: Option<???>,
}

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct AssignedPlan {
    assigned_date_time: String,
    capability_status: String,
    service: String,
    service_plan_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "@odata.type")]
pub(crate) enum GroupOrRole {
    #[serde(rename = "#microsoft.graph.group")]
    Group(Group),
    #[serde(rename = "#microsoft.graph.directoryRole")]
    Role(DirectoryRole),
}

impl<'a> User<'a> {
    #[cfg(test)]
    pub fn new(id: String, display_name: String) -> Self {
        Self {
            id,
            assigned_plans: None,
            //business_phones: None,
            display_name: Some(display_name),
            given_name: None,
            //job_title: None,
            mail: None,
            //mobile_phone: None,
            //office_location: None,
            //preferred_language: None,
            surname: None,
            user_principal_name: None,
            sign_in_activity: None,
            account_enabled: None,
            transitive_member_of: None,
            conditional_access_policies: None,
            is_privileged: None,
        }
    }

    pub fn groups(&self) -> Groups {
        self.transitive_member_of
            .as_ref()
            .unwrap()
            .iter()
            .filter_map(|dir_object| match dir_object {
                GroupOrRole::Group(group) => Some(group),
                GroupOrRole::Role(_) => None,
            })
            .collect::<Groups>()
    }

    pub fn roles(&self) -> DirectoryRoles {
        self.transitive_member_of
            .as_ref()
            .unwrap()
            .iter()
            .filter_map(|dir_object| match dir_object {
                GroupOrRole::Group(_) => None,
                GroupOrRole::Role(role) => Some(role),
            })
            .collect::<DirectoryRoles>()
    }

    pub fn set_is_privileged(&mut self, role_definitions: &RoleDefinitions) {
        for role in self.roles().value.iter() {
            match role_definitions.value.get(dbg!(&role.role_template_id)) {
                Some(role_definition) => {
                    if *role_definition.is_privileged.as_ref().unwrap_or(&false) {
                        self.is_privileged = Some(true);
                        return;
                    }
                }
                None => continue,
            }
        }
        self.is_privileged = Some(false)
    }
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Users<'a> {
    pub value: Vec<User<'a>>,
}

impl<'a> Users<'a> {
    pub fn new() -> Self {
        Self { value: Vec::new() }
    }

    pub fn process_caps(&mut self, caps: &'a ConditionalAccessPolicies) {
        for user in self.value.iter_mut() {
            let mut affected_caps = vec![];
            for cap in caps.value.iter() {
                if cap.affects_user(user) {
                    affected_caps.push(cap.to_user_conditional_access_policy())
                }
            }
            user.conditional_access_policies = Some(affected_caps)
        }
    }

    pub fn set_is_privileged(&mut self, role_definitions: &RoleDefinitions) -> () {
        for user in self.value.iter_mut() {
            user.set_is_privileged(role_definitions);
        }
    }
}

impl<'a> ToHecEventss<'a> for Users<'a> {
    type Item = User<'a>;
    fn source() -> &'static str {
        "msgraph"
    }

    fn sourcetype() -> &'static str {
        "SSPHP.AAD.user"
    }
    fn collection(&'a self) -> &'a [User<'a>] {
        &self.value[..]
    }
}

use std::ops::Deref;
impl<'a> Deref for Users<'a> {
    type Target = [User<'a>];

    fn deref(&self) -> &Self::Target {
        &self.value[..]
    }
}
