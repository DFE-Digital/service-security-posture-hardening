use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Serialize, Deserialize)]
pub struct RoleSchedules {
    pub inner: Vec<RoleSchedule>,
}

impl RoleSchedules {
    pub(crate) fn affects_users(&self) -> impl Iterator<Item = &RoleSchedule> {
        self.inner.iter().filter(|role| {
            role.principal
                .as_ref()
                .map(|principal| matches!(principal.odata_type, PrincipalType::User))
                .unwrap_or_default()
        })
    }

    pub(crate) fn affects_groups(&self) -> impl Iterator<Item = &RoleSchedule> {
        self.inner.iter().filter(|role| {
            role.principal
                .as_ref()
                .map(|principal| matches!(principal.odata_type, PrincipalType::Group))
                .unwrap_or_default()
        })
    }

    /// Role schedules held by a service principal directly.
    ///
    /// `PrincipalType::ServicePrincipal` has been in the enum since the schedules were first
    /// deserialised, so these rows were always being parsed - nothing consumed them, because
    /// only users were enriched. `ServicePrincipalsMap::add_pim_membership` is the consumer.
    pub(crate) fn affects_service_principals(&self) -> impl Iterator<Item = &RoleSchedule> {
        self.inner.iter().filter(|role| {
            role.principal
                .as_ref()
                .map(|principal| matches!(principal.odata_type, PrincipalType::ServicePrincipal))
                .unwrap_or_default()
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
//#[serde(deny_unknown_fields)]
pub struct RoleSchedule {
    activated_using: Option<ActivatedUsing>,
    app_scope: Option<String>,
    app_scope_id: Option<String>,
    assignment_type: AssignmentType,
    created_date_time: Option<String>,
    created_using: Option<String>,
    directory_scope: DirectoryScope,
    directory_scope_id: Option<String>,
    end_date_time: Option<String>,
    id: String,
    member_type: MemberType,
    pub(crate) principal: Option<Principal>,
    ///////
    pub(crate) principal_id: String,
    ////////
    pub(crate) role_definition_id: String,
    role_assignment_origin_id: Option<String>,
    role_assignment_schedule_id: Option<String>,
    schedule_info: Option<ScheduleInfo>,
    start_date_time: Option<String>,
    status: Option<ScheduleStatus>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
//#[serde(deny_unknown_fields)]
struct ActivatedUsing {
    app_scope_id: Option<String>,
    directory_scope_id: String,
    end_date_time: Option<String>,
    id: String,
    member_type: MemberType,
    principal_id: String,
    role_definition_id: String,
    role_eligibility_schedule_id: String,
    start_date_time: String,
}

#[derive(Debug, Serialize, Deserialize)]
enum AssignmentType {
    Activated,
    Assigned,
}
#[derive(Debug, Serialize, Deserialize)]
enum MemberType {
    Direct,
    Group,
}

#[derive(Debug, Serialize, Deserialize)]
struct DirectoryScope {}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Principal {
    #[serde(rename = "@odata.type")]
    pub(crate) odata_type: PrincipalType,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
struct ScheduleInfo {
    expiration: Expiration,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
struct Expiration {
    duration: Option<String>,
    end_date_time: Option<String>,
    r#type: ExpirationType,
    recurrence: Option<String>,
    start_date_time: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
enum ExpirationType {
    AfterDateTime,
    NoExpiration,
}

#[derive(Debug, Serialize, Deserialize)]
enum ScheduleStatus {
    Provisioned,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) enum PrincipalType {
    #[serde(rename = "#microsoft.graph.group")]
    Group,
    #[serde(rename = "#microsoft.graph.servicePrincipal")]
    ServicePrincipal,
    #[serde(rename = "#microsoft.graph.user")]
    User,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One schedule per principal kind, so each filter can be checked for what it lets through
    /// AND what it keeps out.
    fn schedules() -> RoleSchedules {
        let inner: Vec<serde_json::Value> = [
            ("#microsoft.graph.user", "user-1"),
            ("#microsoft.graph.group", "group-1"),
            ("#microsoft.graph.servicePrincipal", "sp-1"),
            ("#microsoft.graph.servicePrincipal", "sp-2"),
        ]
        .iter()
        .enumerate()
        .map(|(i, (odata_type, principal_id))| {
            serde_json::json!({
                "assignmentType": "Assigned",
                "directoryScope": {},
                "id": format!("schedule-{i}"),
                "memberType": "Direct",
                "principal": { "@odata.type": odata_type },
                "principalId": principal_id,
                "roleDefinitionId": "some-role-id"
            })
        })
        .collect();

        serde_json::from_value(serde_json::json!({ "inner": inner }))
            .expect("role schedules should deserialize")
    }

    fn principal_ids<'a>(iter: impl Iterator<Item = &'a RoleSchedule>) -> Vec<&'a str> {
        iter.map(|schedule| schedule.principal_id.as_str())
            .collect()
    }

    #[test]
    fn affects_service_principals_selects_only_service_principals() {
        let schedules = schedules();
        assert_eq!(
            principal_ids(schedules.affects_service_principals()),
            vec!["sp-1", "sp-2"]
        );
    }

    /// The two pre-existing filters must be unchanged by the new one.
    #[test]
    fn the_other_filters_are_unaffected() {
        let schedules = schedules();
        assert_eq!(principal_ids(schedules.affects_users()), vec!["user-1"]);
        assert_eq!(principal_ids(schedules.affects_groups()), vec!["group-1"]);
    }

    /// A schedule carrying no `principal` object is skipped by every filter rather than
    /// defaulting into one of them.
    #[test]
    fn a_schedule_with_no_principal_is_in_no_filter() {
        let schedules: RoleSchedules = serde_json::from_value(serde_json::json!({
            "inner": [{
                "assignmentType": "Assigned",
                "directoryScope": {},
                "id": "schedule-no-principal",
                "memberType": "Direct",
                "principalId": "mystery-principal",
                "roleDefinitionId": "some-role-id"
            }]
        }))
        .expect("role schedules should deserialize");

        assert!(schedules.affects_users().next().is_none());
        assert!(schedules.affects_groups().next().is_none());
        assert!(schedules.affects_service_principals().next().is_none());
    }
}
