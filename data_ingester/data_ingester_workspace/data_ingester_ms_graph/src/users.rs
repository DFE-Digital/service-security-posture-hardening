use crate::conditional_access_policies::ConditionalAccessPolicies;
use crate::conditional_access_policies::UserConditionalAccessPolicy;
use crate::directory_roles::DirectoryRole;
use crate::directory_roles::DirectoryRoles;
use crate::groups::Group;
use crate::groups::Groups;
use crate::role_assignment_schedule::RoleSchedules;
use crate::roles::RoleDefinitions;
use crate::roles::RoleDefinitions as EntraRoleDefinitions;
use anyhow::Context;
use anyhow::Result;
use azure_mgmt_authorization::package_2022_04_01::models::role_assignment_properties::PrincipalType;
use data_ingester_azure_rest::azure_rest::RoleAssignments as AzureRoleAssignments;
use data_ingester_azure_rest::azure_rest::RoleDefinitions as AzureRoleDefinitions;
use data_ingester_splunk::splunk::ToHecEvents;
use regex::Regex;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;
use serde_with::skip_serializing_none;
use std::collections::HashMap;
use std::ops::Deref;
use tracing::warn;

// https://learn.microsoft.com/en-us/graph/api/resources/user?view=graph-rest-1.0
//
// FIELD ORDER IS LOAD-BEARING. serde serialises in declaration order, and Splunk's
// search-time JSON extraction stops after `[kv] maxchars` — 10,240 characters by
// default. Any field past that point is never extracted, so the event is present and
// keyword-searchable but invisible to every field-based search.
//
// Measured 2026-09-30 with `assignedPlans` serialising first: 10,150 of 42,594 user
// records, 23.8%, yielded no fields at all. Identifiers first, then the unbounded
// collections, keeps every field a search filters on inside the first few hundred bytes.
// Do not reorder these into alphabetical or any other order.
#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct User<'a> {
    // Identifiers and scalars: always within the extraction window.
    pub(crate) id: String,
    pub(crate) display_name: Option<String>,
    user_principal_name: Option<String>,
    pub account_enabled: Option<bool>,
    is_privileged: Option<bool>,
    user_type: Option<String>,
    mail: Option<String>,
    given_name: Option<String>,
    surname: Option<String>,
    description: Option<String>,
    on_premises_sam_account_name: Option<String>,
    pub(crate) on_premises_sync_enabled: Option<bool>,
    // Requires scope: AuditLog.Read.All
    sign_in_activity: Option<Value>,

    // Not selected from Graph:
    // business_phones: Option<Vec<String>>,
    // job_title: Option<String>,
    // mobile_phone: Option<String>,
    // office_location: Option<String>,
    // preferred_language: Option<String>,

    // Unbounded collections: these are what push a record past the extraction limit,
    // so they serialise last, least-used first.
    #[serde(skip_deserializing)]
    pub azure_roles: Option<UserAzureRoles>,
    #[serde(skip_deserializing)]
    pim_member_of: Option<Vec<GroupOrRole>>,
    #[serde(skip_deserializing)]
    conditional_access_policies: Option<Vec<UserConditionalAccessPolicy<'a>>>,
    pub(crate) transitive_member_of: Option<Vec<GroupOrRole>>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    assigned_plans: Vec<AssignedPlan>,
}

/// Used to represent an AAD users roles in Azure (Cloud) subscriptions
#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct UserAzureRoles {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub privileged_roles: Vec<UserAzureRole>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub roles: Vec<UserAzureRole>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct UserAzureRole {
    pub id: String,
    pub role_name: String,
}

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AssignedPlan {
    assigned_date_time: String,
    // TODO: ignore Deleted & other...
    capability_status: String, //AssignedPlanCapabilityStatus,
    service: String,
    service_plan_id: String,
}

impl AssignedPlan {
    fn is_enabled(&self) -> bool {
        self.capability_status == "Enabled"
        // match self.capability_status {
        //     AssignedPlanCapabilityStatus::Enabled => true,
        //     AssignedPlanCapabilityStatus::Deleted => false,
        // }
    }
}

// #[derive(Debug, Serialize, Deserialize)]
// #[serde(rename_all = "camelCase")]
// enum AssignedPlanCapabilityStatus {
//     Enabled,
//     Deleted,
// }

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "@odata.type")]
pub(crate) enum GroupOrRole {
    #[serde(rename = "#microsoft.graph.group")]
    Group(Group),
    #[serde(rename = "#microsoft.graph.directoryRole")]
    Role(DirectoryRole),
}

impl User<'_> {
    #[cfg(test)]
    pub fn new(id: String, display_name: String) -> Self {
        Self {
            account_enabled: None,
            assigned_plans: vec![],
            azure_roles: None,
            conditional_access_policies: None,
            description: None,
            display_name: Some(display_name),
            given_name: None,
            id,
            is_privileged: None,
            mail: None,
            on_premises_sam_account_name: None,
            on_premises_sync_enabled: None,
            sign_in_activity: None,
            surname: None,
            transitive_member_of: None,
            user_principal_name: None,
            user_type: None,
            pim_member_of: None,
        }
    }

    pub fn groups(&self) -> Groups<'_> {
        let tmo = self.transitive_member_of.as_ref();

        let tmo_groups = tmo.iter().flat_map(|tmo| {
            tmo.iter().filter_map(|dir_object| match dir_object {
                GroupOrRole::Group(group) => Some(group),
                GroupOrRole::Role(_) => None,
            })
        });

        let pmo = self.pim_member_of.as_ref();
        let pmo_groups = pmo.iter().flat_map(|pmo| {
            pmo.iter().filter_map(|dir_object| match dir_object {
                GroupOrRole::Group(group) => Some(group),
                GroupOrRole::Role(_) => None,
            })
        });

        tmo_groups.chain(pmo_groups).collect()
    }

    pub fn roles(&self) -> DirectoryRoles<'_> {
        let tmo = self.transitive_member_of.as_ref();
        let tmo_roles = tmo.iter().flat_map(|tmo| {
            tmo.iter().filter_map(|dir_object| match dir_object {
                GroupOrRole::Group(_) => None,
                GroupOrRole::Role(role) => Some(role),
            })
        });

        let pmo = self.pim_member_of.as_ref();
        let pmo_roles = pmo.iter().flat_map(|tmo| {
            tmo.iter().filter_map(|dir_object| match dir_object {
                GroupOrRole::Group(_) => None,
                GroupOrRole::Role(role) => Some(role),
            })
        });

        tmo_roles.chain(pmo_roles).collect()
    }

    pub fn roles_mut(&mut self) -> Vec<&mut DirectoryRole> {
        if let Some(tmo) = self.transitive_member_of.as_mut() {
            tmo.iter_mut()
                .filter_map(|dir_object| match dir_object {
                    GroupOrRole::Group(_) => None,
                    GroupOrRole::Role(role) => Some(role),
                })
                .collect::<Vec<&mut DirectoryRole>>()
        } else {
            Vec::new()
        }
    }

    pub fn set_is_privileged(&mut self, role_definitions: &EntraRoleDefinitions) {
        let mut is_privileged = false;

        for role in self.roles_mut() {
            match role_definitions.value.get(&role.role_template_id) {
                Some(role_definition) => {
                    let is_role_privileged =
                        *role_definition.is_privileged.as_ref().unwrap_or(&false);
                    if is_role_privileged {
                        role.is_privileged = Some(true);
                        is_privileged = true;
                    }
                }
                None => continue,
            }
        }

        let privileged_azure_roles = self
            .azure_roles
            .as_ref()
            .map(|roles| !roles.privileged_roles.is_empty())
            .unwrap_or(false);

        self.is_privileged = Some(is_privileged || privileged_azure_roles);
    }

    pub fn assigned_plans_remove_deleted(&mut self) {
        self.assigned_plans.retain(|plan| plan.is_enabled());
    }
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct UsersMap<'a> {
    pub(crate) inner: HashMap<String, User<'a>>,
}

impl<'a> UsersMap<'a> {
    pub fn process_caps(&mut self, caps: &'a ConditionalAccessPolicies) {
        for user in self.inner.values_mut() {
            let mut affected_caps = vec![];
            for cap in caps.inner.iter() {
                if cap.affects_user(user) {
                    affected_caps.push(cap.to_user_conditional_access_policy())
                }
            }
            user.conditional_access_policies = Some(affected_caps)
        }
    }

    pub fn set_is_privileged(&mut self, role_definitions: &EntraRoleDefinitions) {
        for user in self.inner.values_mut() {
            user.set_is_privileged(role_definitions);
        }
    }

    pub fn extend_from_users(&mut self, users: Users<'a>) -> Result<()> {
        for user in users.value.into_iter() {
            _ = self.inner.insert(user.id.to_string(), user);
        }
        Ok(())
    }

    #[cfg(test)]
    pub fn extend(&mut self, users: UsersMap<'a>) {
        self.inner.extend(users.inner);
    }

    pub fn add_pim_membership(
        &mut self,
        role_schedules: &RoleSchedules,
        role_definitions: &RoleDefinitions,
    ) {
        for role_schedule in role_schedules.affects_groups() {
            let role_definition_id = &role_schedule.role_definition_id;
            let principal_id = &role_schedule.principal_id;

            let Some(role_definition) = role_definitions.value.get(role_definition_id) else {
                warn!(
                    role_definition_id = role_definition_id,
                    "Unable to find role definition"
                );
                continue;
            };

            let users = self.inner.values_mut().filter(|user| {
                if let Some(transitive_member_of) = &user.transitive_member_of {
                    transitive_member_of
                        .iter()
                        .any(|group_or_role| match group_or_role {
                            GroupOrRole::Group(Group { id, .. }) => id == principal_id,
                            _ => false,
                        })
                } else {
                    false
                }
            });

            let new_role = GroupOrRole::Role(DirectoryRole {
                id: role_definition.id.clone(),
                display_name: role_definition.display_name.clone(),
                role_template_id: role_definition.id.clone(),
                members: None,
                is_privileged: role_definition.is_privileged,
            });

            for user in users {
                if let Some(ref mut pim_member_of) = user.pim_member_of {
                    if pim_member_of.iter().any(|role| match role {
                        GroupOrRole::Role(DirectoryRole { id, .. }) => id == &role_definition.id,
                        _ => false,
                    }) {
                        continue;
                    }
                    pim_member_of.push(new_role.clone());
                } else {
                    user.pim_member_of = Some(vec![new_role.clone()]);
                }
            }
        }

        for role_schedule in role_schedules.affects_users() {
            let role_definition_id = &role_schedule.role_definition_id;
            let principal_id = &role_schedule.principal_id;

            let Some(ref mut user) = self.inner.get_mut(principal_id) else {
                //warn!(principal_id = principal_id, "Unable to find principal");
                continue;
            };

            let Some(role_definition) = role_definitions.value.get(role_definition_id) else {
                warn!(
                    role_definition_id = role_definition_id,
                    "Unable to find role definition"
                );
                continue;
            };

            let new_role = GroupOrRole::Role(DirectoryRole {
                id: role_definition.id.clone(),
                display_name: role_definition.display_name.clone(),
                role_template_id: role_definition.id.clone(),
                members: None,
                is_privileged: role_definition.is_privileged,
            });

            if let Some(ref mut pim_member_of) = user.pim_member_of {
                if pim_member_of.iter().any(|role| match role {
                    GroupOrRole::Role(DirectoryRole { id, .. }) => id == &role_definition.id,
                    _ => false,
                }) {
                    continue;
                }
                pim_member_of.push(new_role);
            } else {
                user.pim_member_of = Some(vec![new_role]);
            }
        }
    }

    // TODO Get and Follow Groups and join table for users.
    pub fn add_azure_roles(
        &mut self,
        role_assignments: &AzureRoleAssignments,
        role_definitions: &AzureRoleDefinitions,
    ) -> Result<()> {
        let admin_roles_regex = Regex::new(r"(?i)(Owner|contributor|admin)")
            .expect("Static regex should always compile");

        for role_assignment in role_assignments.inner.values() {
            match &role_assignment
                .principal_type()
                .context("Principal Type not User")?
            {
                PrincipalType::User => {}
                _ => continue,
            }

            let role_assignment_role_definition_id = &role_assignment
                .role_definition_id()
                .context("No Role definition")?;

            let Some(role_definition) = role_definitions
                .inner
                .get(*role_assignment_role_definition_id)
            else {
                continue;
            };

            let principal_id = &role_assignment.principal_id().context("No Principal ID")?;

            let Some(ref mut user) = self.inner.get_mut(*principal_id) else {
                continue;
            };

            let id = role_definition.id().context("no role id")?.to_string();

            let role_name = role_definition
                .role_name()
                .context("no role name")?
                .to_string();

            // TODO Should this be part of UserAzureRole?
            let priviliged = admin_roles_regex.find(&role_name).is_some();

            let azure_role = UserAzureRole { id, role_name };

            if user.azure_roles.is_none() {
                user.azure_roles = Some(UserAzureRoles::default());
            }

            if priviliged {
                if let Some(ar) = user.azure_roles.as_mut() {
                    ar.privileged_roles.push(azure_role);
                }
            } else if let Some(ar) = user.azure_roles.as_mut() {
                ar.roles.push(azure_role);
            }
        }
        Ok(())
    }
}

impl<'u> ToHecEvents for &UsersMap<'u> {
    type Item = User<'u>;
    fn source(&self) -> &str {
        "msgraph"
    }

    fn sourcetype(&self) -> &str {
        "SSPHP.AAD.user"
    }

    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        Box::new(self.inner.values())
    }

    fn ssphp_run_key(&self) -> &str {
        "azure_users"
    }
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Users<'a> {
    pub value: Vec<User<'a>>,
}

// impl<'a> ToHecEvents for Users<'a> {
//     type Item = User<'a>;
//     fn source() -> &'static str {
//         "msgraph"
//     }

//     fn sourcetype() -> &'static str {
//         "SSPHP.AAD.user"
//     }
//     fn collection(&self) -> Box<dyn Iterator<Item = &Self::Item> + 'a> {
//         Box::new(self.value.iter())
//     }
// }

impl<'a> Deref for Users<'a> {
    type Target = [User<'a>];

    fn deref(&self) -> &Self::Target {
        &self.value[..]
    }
}
