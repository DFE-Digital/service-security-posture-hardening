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
