use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;
use tracing::error;
use tracing::info;

/// The marker every rejected row is logged under, so the rows can be pulled straight out of the
/// collector's logs. Grep this, not the free text around it.
pub(crate) const REJECTED_MARKER: &str = "ROLE_SCHEDULE_REJECTED";

#[derive(Debug, Serialize, Deserialize)]
pub struct RoleSchedules {
    pub inner: Vec<RoleSchedule>,
}

impl RoleSchedules {
    /// Deserialise a page of Graph values, PARTITIONING rather than discarding.
    ///
    /// WHY THIS EXISTS. Both feeds used to parse row by row with a `filter_map` that logged the
    /// error and returned `None`. A row that failed was dropped and the collection carried on, so
    /// one mandatory field Graph does not send removed an entire feed without failing anything,
    /// and nothing downstream could tell a feed that was empty from a feed that had been thrown
    /// away. That is exactly how the eligibility feed was lost: `assignmentType` was mandatory,
    /// Graph sends it on 0 of 54 eligibility instances, and for as long as that was true no
    /// principal was ever recorded as holding privilege through a group.
    ///
    /// So every rejection is now logged in full - the serde error AND the complete raw payload -
    /// under `REJECTED_MARKER`, with a summary line first giving the counts. The point is that a
    /// rejected row can be recovered from the logs, read, and the struct corrected, instead of
    /// the loss being invisible. `Option` only cures an ABSENT field; an enum value Graph has not
    /// sent before still fails the row, and this is how we find out.
    ///
    /// Rejections are NOT an error for the caller. A feed of 54 where 2 are rejected is still 52
    /// usable rows, and refusing the lot would be worse than carrying on - but it is now loud.
    pub(crate) fn from_values(feed: &str, values: Vec<Value>) -> Self {
        let total = values.len();
        let mut inner = Vec::with_capacity(total);
        let mut rejected: Vec<(serde_json::Error, Value)> = Vec::new();

        for value in values {
            // Deserialise from a REFERENCE so the payload survives for the log line. The old code
            // moved the value into from_value and so had nothing left to report.
            match RoleSchedule::deserialize(&value) {
                Ok(schedule) => inner.push(schedule),
                Err(error) => rejected.push((error, value)),
            }
        }

        if !rejected.is_empty() {
            error!(
                feed = feed,
                total = total,
                accepted = inner.len(),
                rejected = rejected.len(),
                "{REJECTED_MARKER} summary: rows were rejected and are logged individually below"
            );

            for (error, value) in &rejected {
                error!(
                    feed = feed,
                    error = %error,
                    raw = %value,
                    "{REJECTED_MARKER}"
                );
            }
        } else {
            info!(
                feed = feed,
                total = total,
                "all role schedule rows accepted"
            );
        }

        Self { inner }
    }

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
    /// OPTIONAL BECAUSE ELIGIBILITY INSTANCES DO NOT CARRY IT, and until 2026-10-15 this field
    /// was mandatory, which silently threw away the entire eligibility feed.
    ///
    /// `assignmentType` - Assigned or Activated - describes an ASSIGNMENT. Graph does not send it
    /// on `roleEligibilityScheduleInstances`, because being eligible for a role is not an
    /// assignment of it. Measured 2026-10-15 against the live snapshot: present on 67 of 67
    /// assignment instances and 0 of 54 eligibility instances.
    ///
    /// Both list functions deserialise row by row with a `filter_map` that logs the error and
    /// returns `None`, so a row that fails is DROPPED AND THE COLLECTION CARRIES ON. One
    /// mandatory field Graph does not send therefore removes a whole feed without failing
    /// anything. The consequence: the 38 role-assignable groups that hold directory roles
    /// ELIGIBLY never reached `add_pim_membership`, so no principal was ever recorded as holding
    /// privilege through a group - 29 users looked privileged where the real figure is far
    /// higher, and the privileged population has been under-reported for as long as the
    /// eligibility feed has been collected.
    ///
    /// NOTHING ELSE ON THESE ROWS REJECTS THEM, checked against all 54 real payloads on
    /// 2026-10-15: every one parses as JSON and carries exactly appScope, appScopeId,
    /// directoryScope, directoryScopeId, endDateTime, id, memberType, principal, principalId,
    /// roleDefinition, roleDefinitionId, roleEligibilityScheduleId and startDateTime. There is no
    /// `scheduleInfo` - so `ScheduleInfo`'s `deny_unknown_fields` never fires - and no `status`.
    /// `memberType` is Direct on all 54, and `principal.@odata.type` is present on all 54, 38
    /// group and 16 user. BE AWARE THAT `Option` ONLY FIXES AN ABSENT FIELD: an enum value Graph
    /// has not sent before - a new `MemberType`, `ScheduleStatus` or `ExpirationType` - would
    /// still fail the field, fail the row, and vanish the same silent way.
    assignment_type: Option<AssignmentType>,
    created_date_time: Option<String>,
    created_using: Option<String>,
    /// Left mandatory deliberately. Graph sends it on all 54 eligibility instances, 53 of them as
    /// `{"id": "/"}`, which deserialises into the empty `DirectoryScope` struct because unknown
    /// fields are ignored. An earlier reading of this as absent came from a Splunk check that
    /// looked for `directoryScope.@odata.type` - a subfield only one row carries - and that was a
    /// flaw in the check, not in the data.
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

    /// A good row and a bad row in one page: the good one survives, and the page is not abandoned.
    ///
    /// This is the behaviour that was missing. The old `filter_map` also kept the good row, but it
    /// threw the bad one away with only a one-line error, so there was no way to find out WHAT had
    /// been rejected. `from_values` keeps the raw payload and logs it under `REJECTED_MARKER`.
    #[test]
    fn a_rejected_row_does_not_take_the_page_with_it() {
        let values = vec![
            serde_json::json!({
                "directoryScope": { "id": "/" },
                "directoryScopeId": "/",
                "id": "good-row",
                "memberType": "Direct",
                "principal": { "@odata.type": "#microsoft.graph.group" },
                "principalId": "privileged-group-id",
                "roleDefinitionId": "global-admin-role-id"
            }),
            // memberType is an enum of Direct | Group. "Inherited" is not a variant, so this row
            // fails - and `Option` would not save it, because the field is PRESENT and its VALUE
            // is unrecognised. That is the residual failure mode this logging exists to surface.
            serde_json::json!({
                "directoryScope": { "id": "/" },
                "directoryScopeId": "/",
                "id": "bad-row",
                "memberType": "Inherited",
                "principal": { "@odata.type": "#microsoft.graph.group" },
                "principalId": "another-group-id",
                "roleDefinitionId": "global-admin-role-id"
            }),
        ];

        let schedules = RoleSchedules::from_values("test-feed", values);

        assert_eq!(
            schedules.inner.len(),
            1,
            "the good row must survive a rejected sibling"
        );
        assert_eq!(
            principal_ids(schedules.inner.iter()),
            vec!["privileged-group-id"]
        );
    }

    /// A page where every row is rejected returns empty rather than erroring - 0 usable rows is
    /// still a valid answer for the caller - but it is logged loudly, which is the whole point.
    #[test]
    fn a_page_of_all_rejected_rows_returns_empty() {
        let values = vec![serde_json::json!({ "id": "nothing-else-at-all" })];

        let schedules = RoleSchedules::from_values("test-feed", values);

        assert!(schedules.inner.is_empty());
    }

    /// An empty page is not a rejection.
    #[test]
    fn an_empty_page_is_accepted() {
        let schedules = RoleSchedules::from_values("test-feed", vec![]);
        assert!(schedules.inner.is_empty());
    }

    /// The grep marker is fixed, because log extraction depends on it.
    #[test]
    fn the_rejection_marker_is_stable() {
        assert_eq!(REJECTED_MARKER, "ROLE_SCHEDULE_REJECTED");
    }

    /// REGRESSION TEST for the defect fixed on 2026-10-15.
    ///
    /// This is the shape Graph actually sends on `roleEligibilityScheduleInstances`: NO
    /// `assignmentType`, and no populated `directoryScope` object. Both were mandatory on
    /// `RoleSchedule` until 2026-10-15, so every row like this failed to deserialize and was
    /// dropped by the `filter_map` in `list_role_eligibility_schedules` - logged, and the
    /// collection carried on short.
    ///
    /// The consequence was that the 38 role-assignable groups holding directory roles ELIGIBLY
    /// never reached `add_pim_membership`, so privilege held through a group was invisible.
    #[test]
    fn an_eligibility_instance_deserializes() {
        // This is the exact key set Graph sends, taken from all 54 live payloads on 2026-10-15.
        // The values are synthetic - real object ids and group descriptions are not committed -
        // but the SHAPE is faithful: no `assignmentType`, no `scheduleInfo`, no `status`, and
        // `directoryScope` populated as {"id": "/"}, which is how 53 of the 54 arrive.
        let schedules: RoleSchedules = serde_json::from_value(serde_json::json!({
            "inner": [{
                "appScope": null,
                "appScopeId": null,
                "directoryScope": { "id": "/" },
                "directoryScopeId": "/",
                "endDateTime": null,
                "id": "eligibility-instance-1",
                "memberType": "Direct",
                "principal": {
                    "@odata.type": "#microsoft.graph.group",
                    "classification": null,
                    "createdByAppId": "00000000-0000-0000-0000-000000000000",
                    "createdDateTime": "2025-08-15T08:57:56Z",
                    "deletedDateTime": null,
                    "description": "Gives PIM access to a directory role"
                },
                "principalId": "privileged-group-id",
                "roleDefinition": {
                    "description": "",
                    "displayName": "Global Administrator",
                    "id": "global-admin-role-id",
                    "isBuiltIn": true,
                    "isEnabled": true,
                    "resourceScopes": []
                },
                "roleDefinitionId": "global-admin-role-id",
                "roleEligibilityScheduleId": "eligibility-schedule-1",
                "startDateTime": "2025-08-15T08:58:58.463Z"
            }]
        }))
        .expect("an eligibility instance with no assignmentType must deserialize");

        assert_eq!(schedules.inner.len(), 1);
        assert_eq!(
            principal_ids(schedules.affects_groups()),
            vec!["privileged-group-id"],
            "an eligible group must reach affects_groups, or group-held privilege is invisible"
        );
    }

    /// The assignment shape, which always worked, must keep working - `assignmentType` present
    /// and a populated `directoryScope` object.
    #[test]
    fn an_assignment_instance_still_deserializes() {
        let schedules: RoleSchedules = serde_json::from_value(serde_json::json!({
            "inner": [{
                "assignmentType": "Assigned",
                "directoryScope": {
                    "@odata.type": "#microsoft.graph.administrativeUnit",
                    "displayName": "Some Admin Unit",
                    "id": "/administrativeUnits/au-1"
                },
                "directoryScopeId": "/administrativeUnits/au-1",
                "id": "assignment-instance-1",
                "memberType": "Direct",
                "principal": { "@odata.type": "#microsoft.graph.servicePrincipal" },
                "principalId": "sp-1",
                "roleDefinitionId": "groups-admin-role-id"
            }]
        }))
        .expect("an assignment instance must deserialize");

        assert_eq!(
            principal_ids(schedules.affects_service_principals()),
            vec!["sp-1"]
        );
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
