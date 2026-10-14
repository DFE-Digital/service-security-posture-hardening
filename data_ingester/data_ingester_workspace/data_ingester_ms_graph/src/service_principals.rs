use crate::directory_roles::DirectoryRole;
use crate::groups::Group;
use crate::role_assignment_schedule::RoleSchedules;
use crate::roles::RoleDefinitions;
use crate::users::GroupOrRole;
use crate::users::UserAzureRole;
use crate::users::UserAzureRoles;
use anyhow::Context;
use anyhow::Result;
use azure_mgmt_authorization::package_2022_04_01::models::role_assignment_properties::PrincipalType;
use data_ingester_azure_rest::azure_rest::RoleAssignments as AzureRoleAssignments;
use data_ingester_azure_rest::azure_rest::RoleDefinitions as AzureRoleDefinitions;
use data_ingester_splunk::splunk::ToHecEvents;
use regex::Regex;
use serde::Deserialize;
use serde::Serialize;
use serde_with::skip_serializing_none;
use std::collections::HashMap;
use std::ops::Deref;
use tracing::warn;

// https://learn.microsoft.com/en-us/graph/api/resources/serviceprincipal?view=graph-rest-1.0
//
// FIELD ORDER IS LOAD-BEARING, for the same reason it is on `User`. serde serialises in
// declaration order and Splunk's search-time JSON extraction stops after `[kv] maxchars`,
// 10,240 characters by default. Any field past that point is present in the event and
// keyword-searchable, but invisible to every field-based search. On the user feed that was
// measured as 23.8% of records yielding no fields at all.
//
// Identifiers and scalars first, then the unbounded collections. A service principal's
// `appRoles` and `oauth2PermissionScopes` run to tens of thousands of characters for a large
// application and are deliberately NOT collected here - see the comment on the
// `service_principals` entry in ms_graph.toml. Do not reorder these.
#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ServicePrincipal {
    // Identifiers and scalars: always within the extraction window.
    pub(crate) id: String,
    pub(crate) display_name: Option<String>,
    app_id: Option<String>,
    pub account_enabled: Option<bool>,
    service_principal_type: Option<String>,
    app_owner_organization_id: Option<String>,
    sign_in_audience: Option<String>,
    app_role_assignment_required: Option<bool>,
    publisher_name: Option<String>,

    // Unbounded collections: these are what push a record past the extraction limit, so they
    // serialise last, least-used first.
    #[serde(skip_serializing_if = "Option::is_none")]
    tags: Option<Vec<String>>,
    #[serde(skip_deserializing)]
    pub azure_roles: Option<UserAzureRoles>,
    #[serde(skip_deserializing)]
    pim_member_of: Option<Vec<GroupOrRole>>,
    pub(crate) transitive_member_of: Option<Vec<GroupOrRole>>,
}

impl ServicePrincipal {
    /// Record one Azure RBAC role, skipping it if this principal already carries it.
    ///
    /// DEDUPLICATION IS NEEDED HERE AND NOT ON THE USER SIDE, because this is the first place a
    /// role can arrive by two routes: directly, and through more than one group. A principal in
    /// two groups that both hold Owner would otherwise carry Owner twice.
    ///
    /// `UserAzureRole` carries the role id and name but NO SCOPE, so two assignments of the same
    /// role at different scopes are indistinguishable in it and collapse to one here. That is a
    /// limitation of the type rather than of this method, and it is the reason a count of roles
    /// from this feed is not a count of assignments.
    fn push_azure_role(&mut self, azure_role: UserAzureRole, privileged: bool) {
        let roles = self.azure_roles.get_or_insert_with(UserAzureRoles::default);

        if roles
            .privileged_roles
            .iter()
            .chain(roles.roles.iter())
            .any(|existing| existing.id == azure_role.id)
        {
            return;
        }

        if privileged {
            roles.privileged_roles.push(azure_role);
        } else {
            roles.roles.push(azure_role);
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct ServicePrincipalsMap {
    pub(crate) inner: HashMap<String, ServicePrincipal>,
}

impl ServicePrincipalsMap {
    pub fn extend_from_service_principals(
        &mut self,
        service_principals: ServicePrincipals,
    ) -> Result<()> {
        for service_principal in service_principals.value.into_iter() {
            _ = self
                .inner
                .insert(service_principal.id.to_string(), service_principal);
        }
        Ok(())
    }

    #[cfg(test)]
    pub fn extend(&mut self, service_principals: ServicePrincipalsMap) {
        self.inner.extend(service_principals.inner);
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Record the PIM-eligible and PIM-assigned directory roles a service principal holds.
    ///
    /// This is the port of `UsersMap::add_pim_membership` and it answers the question that
    /// motivated the whole feed: WHICH SERVICE PRINCIPALS HOLD ENTRA PRIVILEGE. Measured on
    /// 2026-10-14 against Splunk, there was no way to answer it - `transitiveMemberOf` existed
    /// only on the user feed, `/v1.0/groups` and `azure:aad:group` carry no members at all, and
    /// `/v1.0/servicePrincipals` selects ten attributes with membership not among them. So a
    /// population of privileged principals silently omitted every service principal that holds a
    /// role through a group, which is the case most worth finding: a service principal exists in
    /// order to be used, so one holding privilege it never uses is either an automation that was
    /// never built or a grant that was not legitimate.
    ///
    /// Two halves, because a role schedule names either a group or a principal directly:
    ///
    ///   1. `affects_groups()` - the role is held by a GROUP, so every service principal whose
    ///      `transitive_member_of` contains that group inherits it. This half is the reason
    ///      `get_transitive_members_of_for_service_principals` has to run FIRST.
    ///   2. `affects_service_principals()` - the role is held by the service principal itself.
    ///
    /// Roles already recorded are skipped, so a principal that reaches the same role by both
    /// routes carries it once.
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

            let service_principals = self.inner.values_mut().filter(|service_principal| {
                if let Some(transitive_member_of) = &service_principal.transitive_member_of {
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

            for service_principal in service_principals {
                if let Some(ref mut pim_member_of) = service_principal.pim_member_of {
                    if pim_member_of.iter().any(|role| match role {
                        GroupOrRole::Role(DirectoryRole { id, .. }) => id == &role_definition.id,
                        _ => false,
                    }) {
                        continue;
                    }
                    pim_member_of.push(new_role.clone());
                } else {
                    service_principal.pim_member_of = Some(vec![new_role.clone()]);
                }
            }
        }

        for role_schedule in role_schedules.affects_service_principals() {
            let role_definition_id = &role_schedule.role_definition_id;
            let principal_id = &role_schedule.principal_id;

            let Some(ref mut service_principal) = self.inner.get_mut(principal_id) else {
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

            if let Some(ref mut pim_member_of) = service_principal.pim_member_of {
                if pim_member_of.iter().any(|role| match role {
                    GroupOrRole::Role(DirectoryRole { id, .. }) => id == &role_definition.id,
                    _ => false,
                }) {
                    continue;
                }
                pim_member_of.push(new_role);
            } else {
                service_principal.pim_member_of = Some(vec![new_role]);
            }
        }
    }

    /// Record the Azure RBAC roles a service principal holds, on subscriptions and below.
    ///
    /// This is the port of `UsersMap::add_azure_roles`, and it matters more here than there.
    /// Measured 2026-10-14 from the `SSPHP.azure.role_assignment` snapshot: the Azure RBAC
    /// standing population is roughly 988 service principals against 64 users, with 481 service
    /// principals holding a role whose impact is rated 3. Azure RBAC is overwhelmingly held by
    /// service principals, so a feed that described their Entra roles and not their Azure ones
    /// would miss where most of their privilege actually is.
    ///
    /// IT DOES BOTH ROUTES, DIRECT AND THROUGH A GROUP. The user version only does direct - its
    /// own `// TODO Get and Follow Groups` says so - because until now nothing knew which groups
    /// a principal was in. `transitive_member_of` is fetched before this runs, so the group route
    /// is available, and leaving it out would repeat the exact gap this work exists to close:
    /// privilege held through a group being invisible. The users side is untouched and still
    /// misses it.
    ///
    /// `ForeignGroup` and `Device` assignments are skipped along with `User` ones: a service
    /// principal is never a member of either.
    ///
    /// A GROUP WE CANNOT RESOLVE MATCHES NOBODY RATHER THAN ERRORING, and an empty result for it
    /// therefore means "we cannot see the members", NOT "it has none". That is the cross-tenant
    /// case: many subscriptions are held in other tenancies and reached through Lighthouse, so
    /// ARM reports their role assignments while Entra, collected from one tenancy only, cannot
    /// report the membership of a group that lives in another. Measured 2026-10-14 and SMALL:
    /// 1,768 of 1,789 group assignments name a group we hold, so 21 across 12 groups are
    /// unresolvable. There is no `ForeignGroup` marker to key on - cross-tenant groups arrive as
    /// plain `Group` - so the only test is whether we hold the group at all.
    ///
    /// PRIVILEGE IS JUDGED ON THE ROLE NAME, by the same `(?i)(Owner|contributor|admin)` regex
    /// the user side uses, so the two feeds agree. It is a crude test - it calls "Monitoring
    /// Contributor" privileged and misses a custom role with write permissions and a harmless
    /// name - and the privilege family's own inventory, which scores all 505 built-in Azure roles
    /// from their permission lists, is the better judge downstream. This flag is a convenience,
    /// not the authority.
    pub fn add_azure_roles(
        &mut self,
        role_assignments: &AzureRoleAssignments,
        role_definitions: &AzureRoleDefinitions,
    ) -> Result<()> {
        let admin_roles_regex = Regex::new(r"(?i)(Owner|contributor|admin)")
            .expect("Static regex should always compile");

        for role_assignment in role_assignments.inner.values() {
            let principal_type = role_assignment
                .principal_type()
                .context("No Principal Type")?;

            let via_group = match principal_type {
                PrincipalType::ServicePrincipal => false,
                PrincipalType::Group => true,
                _ => continue,
            };

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

            let id = role_definition.id().context("no role id")?.to_string();

            let role_name = role_definition
                .role_name()
                .context("no role name")?
                .to_string();

            let privileged = admin_roles_regex.find(&role_name).is_some();

            if via_group {
                let service_principals = self.inner.values_mut().filter(|service_principal| {
                    if let Some(transitive_member_of) = &service_principal.transitive_member_of {
                        transitive_member_of
                            .iter()
                            .any(|group_or_role| match group_or_role {
                                GroupOrRole::Group(Group { id, .. }) => id == *principal_id,
                                _ => false,
                            })
                    } else {
                        false
                    }
                });

                for service_principal in service_principals {
                    service_principal.push_azure_role(
                        UserAzureRole {
                            id: id.clone(),
                            role_name: role_name.clone(),
                        },
                        privileged,
                    );
                }
            } else {
                let Some(ref mut service_principal) = self.inner.get_mut(*principal_id) else {
                    continue;
                };

                service_principal.push_azure_role(UserAzureRole { id, role_name }, privileged);
            }
        }
        Ok(())
    }
}

impl ToHecEvents for &ServicePrincipalsMap {
    type Item = ServicePrincipal;
    fn source(&self) -> &str {
        "msgraph"
    }

    fn sourcetype(&self) -> &str {
        "SSPHP.AAD.service_principal"
    }

    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        Box::new(self.inner.values())
    }

    // The SSPHP_RUN key stays "azure_users" even though the collecting function is now
    // `azure_iam`. It is a DATA-PLANE string: `set_ssphp_run_with_seconds` registers the run id
    // under this key and every event stamped by this collection run has to look it up by the
    // same name. Renaming it would change SSPHP_RUN in Splunk and break the latest-run
    // subsearches the detections use. Users and service principals are one run and share it.
    fn ssphp_run_key(&self) -> &str {
        "azure_users"
    }
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct ServicePrincipals {
    pub value: Vec<ServicePrincipal>,
}

impl Deref for ServicePrincipals {
    type Target = [ServicePrincipal];
    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::role_assignment_schedule::RoleSchedules;
    use crate::roles::RoleDefinitions;

    const GA: &str = "global-admin-role-id";
    const READER: &str = "message-center-reader-role-id";
    const PRIV_GROUP: &str = "privileged-group-id";
    const OTHER_GROUP: &str = "some-other-group-id";

    fn role_definitions() -> RoleDefinitions {
        serde_json::from_value(serde_json::json!({
            "value": [
                { "id": GA, "displayName": "Global Administrator", "isPrivileged": true },
                { "id": READER, "displayName": "Message Center Reader", "isPrivileged": false }
            ]
        }))
        .expect("role definitions should deserialize")
    }

    /// One PIM schedule granting `role_definition_id` to `principal_id`, whose `@odata.type`
    /// decides which half of `add_pim_membership` picks it up.
    fn schedules(entries: &[(&str, &str, &str)]) -> RoleSchedules {
        let inner: Vec<serde_json::Value> = entries
            .iter()
            .enumerate()
            .map(|(i, (odata_type, principal_id, role_definition_id))| {
                serde_json::json!({
                    "assignmentType": "Assigned",
                    "directoryScope": {},
                    "id": format!("schedule-{i}"),
                    "memberType": "Direct",
                    "principal": { "@odata.type": odata_type },
                    "principalId": principal_id,
                    "roleDefinitionId": role_definition_id
                })
            })
            .collect();

        serde_json::from_value(serde_json::json!({ "inner": inner }))
            .expect("role schedules should deserialize")
    }

    /// A service principal, optionally a transitive member of some groups.
    fn service_principal(id: &str, groups: &[&str]) -> serde_json::Value {
        let memberships: Vec<serde_json::Value> = groups
            .iter()
            .map(|gid| serde_json::json!({ "@odata.type": "#microsoft.graph.group", "id": gid }))
            .collect();

        serde_json::json!({
            "id": id,
            "displayName": id,
            "accountEnabled": true,
            "transitiveMemberOf": memberships
        })
    }

    fn map_of(service_principals: &[serde_json::Value]) -> ServicePrincipalsMap {
        let parsed: ServicePrincipals =
            serde_json::from_value(serde_json::json!({ "value": service_principals }))
                .expect("service principals should deserialize");

        let mut map = ServicePrincipalsMap::default();
        map.extend_from_service_principals(parsed)
            .expect("extend should succeed");
        map
    }

    fn pim_role_ids(map: &ServicePrincipalsMap, id: &str) -> Vec<String> {
        map.inner
            .get(id)
            .expect("service principal should be present")
            .pim_member_of
            .as_ref()
            .map(|roles| {
                roles
                    .iter()
                    .filter_map(|role| match role {
                        GroupOrRole::Role(DirectoryRole { id, .. }) => Some(id.clone()),
                        GroupOrRole::Group(_) => None,
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    // ── add_pim_membership ──────────────────────────────────────────────────────────────────

    /// A role held by the service principal itself.
    #[test]
    fn pim_role_held_directly_is_recorded() {
        let mut map = map_of(&[service_principal("sp-1", &[])]);

        map.add_pim_membership(
            &schedules(&[("#microsoft.graph.servicePrincipal", "sp-1", GA)]),
            &role_definitions(),
        );

        assert_eq!(pim_role_ids(&map, "sp-1"), vec![GA.to_owned()]);
    }

    /// THE CASE THIS WHOLE FEED EXISTS FOR: a role held by a GROUP the service principal is a
    /// transitive member of. Nothing in Splunk could answer this before 2026-10-14.
    #[test]
    fn pim_role_held_through_a_group_is_recorded() {
        let mut map = map_of(&[service_principal("sp-1", &[PRIV_GROUP])]);

        map.add_pim_membership(
            &schedules(&[("#microsoft.graph.group", PRIV_GROUP, GA)]),
            &role_definitions(),
        );

        assert_eq!(pim_role_ids(&map, "sp-1"), vec![GA.to_owned()]);
    }

    /// A principal in a different group must not inherit the role.
    #[test]
    fn pim_role_does_not_leak_to_non_members() {
        let mut map = map_of(&[
            service_principal("sp-in", &[PRIV_GROUP]),
            service_principal("sp-out", &[OTHER_GROUP]),
            service_principal("sp-none", &[]),
        ]);

        map.add_pim_membership(
            &schedules(&[("#microsoft.graph.group", PRIV_GROUP, GA)]),
            &role_definitions(),
        );

        assert_eq!(pim_role_ids(&map, "sp-in"), vec![GA.to_owned()]);
        assert!(pim_role_ids(&map, "sp-out").is_empty());
        assert!(pim_role_ids(&map, "sp-none").is_empty());
    }

    /// Reaching the same role directly AND through a group records it once.
    #[test]
    fn pim_role_reached_by_both_routes_is_recorded_once() {
        let mut map = map_of(&[service_principal("sp-1", &[PRIV_GROUP])]);

        map.add_pim_membership(
            &schedules(&[
                ("#microsoft.graph.group", PRIV_GROUP, GA),
                ("#microsoft.graph.servicePrincipal", "sp-1", GA),
            ]),
            &role_definitions(),
        );

        assert_eq!(pim_role_ids(&map, "sp-1"), vec![GA.to_owned()]);
    }

    /// A schedule naming a USER must not touch service principals at all.
    #[test]
    fn user_schedules_are_ignored() {
        let mut map = map_of(&[service_principal("sp-1", &[])]);

        map.add_pim_membership(
            &schedules(&[("#microsoft.graph.user", "sp-1", GA)]),
            &role_definitions(),
        );

        assert!(pim_role_ids(&map, "sp-1").is_empty());
    }

    /// A group we hold no membership for matches nobody rather than erroring. That is the
    /// cross-tenant case - measured 2026-10-14 as 21 of 1,789 Azure group assignments - and an
    /// empty result here means "we cannot see the members", NOT "it has none".
    #[test]
    fn unresolvable_group_matches_nobody_without_erroring() {
        let mut map = map_of(&[service_principal("sp-1", &[])]);

        map.add_pim_membership(
            &schedules(&[("#microsoft.graph.group", "a-group-in-another-tenancy", GA)]),
            &role_definitions(),
        );

        assert!(pim_role_ids(&map, "sp-1").is_empty());
    }

    // ── add_azure_roles ────────────────────────────────────────────────────────────────────

    /// Azure RBAC assignments. `principal_type` decides direct versus via-group.
    fn azure_assignments(entries: &[(&str, &str, &str)]) -> AzureRoleAssignments {
        let mut inner = serde_json::Map::new();
        for (i, (principal_type, principal_id, role_definition_id)) in entries.iter().enumerate() {
            _ = inner.insert(
                format!("assignment-{i}"),
                serde_json::json!({
                    "properties": {
                        "roleDefinitionId": role_definition_id,
                        "principalId": principal_id,
                        "principalType": principal_type,
                        "scope": "/subscriptions/sub-1"
                    }
                }),
            );
        }

        serde_json::from_value(serde_json::json!({ "inner": inner }))
            .expect("azure role assignments should deserialize")
    }

    fn azure_definitions(entries: &[(&str, &str)]) -> AzureRoleDefinitions {
        let mut inner = serde_json::Map::new();
        for (id, role_name) in entries {
            _ = inner.insert(
                (*id).to_owned(),
                serde_json::json!({ "id": id, "properties": { "roleName": role_name } }),
            );
        }

        serde_json::from_value(serde_json::json!({ "inner": inner }))
            .expect("azure role definitions should deserialize")
    }

    fn azure_role_names(map: &ServicePrincipalsMap, id: &str) -> (Vec<String>, Vec<String>) {
        let roles = map
            .inner
            .get(id)
            .expect("service principal should be present")
            .azure_roles
            .as_ref();

        match roles {
            Some(roles) => (
                roles
                    .privileged_roles
                    .iter()
                    .map(|role| role.role_name.clone())
                    .collect(),
                roles
                    .roles
                    .iter()
                    .map(|role| role.role_name.clone())
                    .collect(),
            ),
            None => (vec![], vec![]),
        }
    }

    /// An Azure role assigned to the service principal itself, split by the name test.
    #[test]
    fn azure_role_held_directly_is_recorded_and_split_by_privilege() {
        let mut map = map_of(&[service_principal("sp-1", &[])]);

        map.add_azure_roles(
            &azure_assignments(&[
                ("ServicePrincipal", "sp-1", "owner-def"),
                ("ServicePrincipal", "sp-1", "reader-def"),
            ]),
            &azure_definitions(&[("owner-def", "Owner"), ("reader-def", "Reader")]),
        )
        .expect("add_azure_roles should succeed");

        let (privileged, ordinary) = azure_role_names(&map, "sp-1");
        assert_eq!(privileged, vec!["Owner".to_owned()]);
        assert_eq!(ordinary, vec!["Reader".to_owned()]);
    }

    /// An Azure role assigned to a GROUP reaches its members. The user-side version of this
    /// method does NOT do this - its own `// TODO Get and Follow Groups` says so - because until
    /// now nothing knew group membership.
    #[test]
    fn azure_role_held_through_a_group_is_recorded() {
        let mut map = map_of(&[
            service_principal("sp-in", &[PRIV_GROUP]),
            service_principal("sp-out", &[OTHER_GROUP]),
        ]);

        map.add_azure_roles(
            &azure_assignments(&[("Group", PRIV_GROUP, "owner-def")]),
            &azure_definitions(&[("owner-def", "Owner")]),
        )
        .expect("add_azure_roles should succeed");

        assert_eq!(azure_role_names(&map, "sp-in").0, vec!["Owner".to_owned()]);
        assert_eq!(azure_role_names(&map, "sp-out"), (vec![], vec![]));
    }

    /// The same role reached directly and through two groups is recorded once.
    #[test]
    fn azure_role_is_deduplicated_across_routes() {
        let mut map = map_of(&[service_principal("sp-1", &[PRIV_GROUP, OTHER_GROUP])]);

        map.add_azure_roles(
            &azure_assignments(&[
                ("Group", PRIV_GROUP, "owner-def"),
                ("Group", OTHER_GROUP, "owner-def"),
                ("ServicePrincipal", "sp-1", "owner-def"),
            ]),
            &azure_definitions(&[("owner-def", "Owner")]),
        )
        .expect("add_azure_roles should succeed");

        assert_eq!(azure_role_names(&map, "sp-1").0, vec!["Owner".to_owned()]);
    }

    /// User and Device assignments are not a service principal's business.
    #[test]
    fn azure_user_assignments_are_ignored() {
        let mut map = map_of(&[service_principal("sp-1", &[])]);

        map.add_azure_roles(
            &azure_assignments(&[("User", "sp-1", "owner-def")]),
            &azure_definitions(&[("owner-def", "Owner")]),
        )
        .expect("add_azure_roles should succeed");

        assert_eq!(azure_role_names(&map, "sp-1"), (vec![], vec![]));
    }

    /// An assignment whose role definition is not in the snapshot is skipped, not guessed at.
    #[test]
    fn azure_assignment_with_no_definition_is_skipped() {
        let mut map = map_of(&[service_principal("sp-1", &[])]);

        map.add_azure_roles(
            &azure_assignments(&[("ServicePrincipal", "sp-1", "a-definition-we-do-not-have")]),
            &azure_definitions(&[("owner-def", "Owner")]),
        )
        .expect("add_azure_roles should succeed");

        assert_eq!(azure_role_names(&map, "sp-1"), (vec![], vec![]));
    }

    /// The HEC contract: the sourcetype is new, and the SSPHP_RUN key is deliberately the same
    /// one the user feed uses, because users and service principals are one collection run.
    #[test]
    fn hec_contract_is_stable() {
        let map = map_of(&[service_principal("sp-1", &[])]);
        let map_ref = &map;

        assert_eq!(map_ref.source(), "msgraph");
        assert_eq!(map_ref.sourcetype(), "SSPHP.AAD.service_principal");
        assert_eq!(map_ref.ssphp_run_key(), "azure_users");
        assert_eq!(map_ref.collection().count(), 1);
    }

    /// Identifiers must serialise before the unbounded collections, or Splunk's JSON extraction
    /// stops at `[kv] maxchars` and the record yields no fields at all - measured on the user
    /// feed as 23.8% of records.
    #[test]
    fn identifiers_serialise_before_the_unbounded_collections() {
        let map = map_of(&[service_principal("sp-1", &[PRIV_GROUP])]);
        let service_principal = map.inner.get("sp-1").expect("present");
        let json = serde_json::to_string(service_principal).expect("serialises");

        let id_at = json.find("\"id\"").expect("id present");
        let memberships_at = json
            .find("\"transitiveMemberOf\"")
            .expect("transitiveMemberOf present");

        assert!(
            id_at < memberships_at,
            "identifiers must come before transitiveMemberOf: {json}"
        );
    }
}
