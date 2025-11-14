use std::collections::HashSet;

use data_ingester_splunk::splunk::ToHecEvents;
use serde::Deserializer;

use crate::users::User;
use serde::Deserialize;
use serde::Serialize;
use serde_with::skip_serializing_none;

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct ConditionalAccessPolicy {
    pub(crate) id: String,
    display_name: Option<String>,
    state: Option<String>,
    conditions: ConditionalAccessPolicyConditions,
    grant_controls: Option<serde_json::Value>,
    session_controls: Option<serde_json::Value>,
    created_date_time: Option<String>,
    modified_date_time: Option<String>,
    template_id: Option<String>,

    pub has_untrusted_conditions: Option<Vec<String>>,
}

impl ConditionalAccessPolicy {
    pub fn affects_user(&self, user: &User) -> bool {
        let condition_users = &self.conditions.users;

        // -- Check excludes

        // Users
        if condition_users.exclude_users.contains("All") {
            return false;
        }
        if condition_users.exclude_users.contains(&user.id) {
            return false;
        }

        // Groups
        if condition_users.exclude_groups.contains("All") {
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
        if condition_users.exclude_roles.contains("All") {
            return false;
        }
        for role_id in user.roles().ids() {
            if condition_users.exclude_roles.contains(role_id) {
                return false;
            }
        }

        for role_template_id in user.roles().template_ids() {
            if condition_users.exclude_roles.contains(role_template_id) {
                return false;
            }
        }

        // -- Check includes

        // Users
        if condition_users.include_users.contains("All") {
            return true;
        }
        if condition_users.include_users.contains(&user.id) {
            return true;
        }

        // Groups
        if condition_users.include_groups.contains("All") {
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
        if condition_users.include_roles.contains("All") {
            return true;
        }

        for role_id in user.roles().ids() {
            if condition_users.include_roles.contains(role_id) {
                return true;
            }
        }

        for role_template_id in user.roles().template_ids() {
            if condition_users.include_roles.contains(role_template_id) {
                return true;
            }
        }

        // CAP does not apply
        false
    }

    pub fn to_user_conditional_access_policy(&self) -> UserConditionalAccessPolicy<'_> {
        UserConditionalAccessPolicy {
            id: self.id.as_str(),
            display_name: self.display_name.as_deref(),
            state: self.state.as_deref(),
        }
    }

    pub fn has_other_conditions(&self) -> Vec<String> {
        let mut result = vec![];

        let conditions = &self.conditions;

        if !conditions.applications.exclude_applications.is_empty() {
            result.push("conditions.applications.exclude_applications".to_string());
        }

        if !conditions.applications.include_user_actions.is_empty() {
            result.push("conditions.applications.include_user_actions".to_string());
        }

        if !conditions
            .applications
            .include_authentication_context_class_references
            .is_empty()
        {
            result.push("conditions.applications.include_user_actions".to_string());
        }

        if conditions.authentication_flows.is_some() {
            result.push("conditions.authentication_flows".to_string());
        }

        if conditions.client_applications.is_some() {
            result.push("conditions.client_applications".to_string());
        }

        if conditions.devices.is_some() {
            result.push("conditions.devices".to_string());
        }

        if conditions.insider_risk_levels.is_some() {
            result.push("conditions.insider_risk_levels".to_string());
        }

        if conditions.platforms.is_some() {
            result.push("conditions.platforms".to_string());
        }

        if !conditions.service_principal_risk_levels.is_empty() {
            result.push("conditions.service_principal_risk_levels".to_string());
        }

        if !conditions.sign_in_risk_levels.is_empty() {
            result.push("conditions.sign_in_risk_levels".to_string());
        }

        if !conditions.user_risk_levels.is_empty() {
            result.push("conditions.user_risk_levels".to_string());
        }

        result
    }
}

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct ConditionalAccessPolicyConditions {
    applications: Applications,
    authentication_flows: Option<AuthenticationFlows>,
    client_applications: Option<ClientApplications>,

    client_app_types: Vec<ClientAppTypes>,
    devices: Option<Devices>,
    insider_risk_levels: Option<InsiderRiskLevels>,
    locations: Option<Locations>,
    platforms: Option<Platforms>,
    service_principal_risk_levels: Vec<RiskLevel>,
    sign_in_risk_levels: Vec<RiskLevel>,
    user_risk_levels: Vec<RiskLevel>,
    users: ConditionalAccessPolicyConditionsUsers,
}

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
enum ClientAppTypes {
    All,
    Browser,
    MobileAppsAndDesktopClients,
    ExchangeActiveSync,
    EasSupported,
    EasUnsupported,
    #[default]
    Other,
}

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
struct Applications {
    exclude_applications: Vec<String>,
    include_applications: Vec<String>,
    application_filter: Option<AccessFilter>,
    include_user_actions: Vec<String>,
    include_authentication_context_class_references: Vec<String>,
}

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
struct AccessFilter {
    mode: ApplicationFilterMode,
    rule: String,
}

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
enum ApplicationFilterMode {
    Include,
    #[default]
    Exclude,
}

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
struct AuthenticationFlows {
    transfer_methods: AuthenticationFlowsTransferMethods,
}

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
enum AuthenticationFlowsTransferMethods {
    None,
    DeviceCodeFlow,
    AuthenticationTransfer,
    #[default]
    UnknownFutureValue,
}

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
struct ClientApplications {
    exclude_service_principals: Vec<String>,
    include_service_principals: Vec<String>,
    service_principal_filter: AccessFilter,
}

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
struct Devices {
    device_filter: AccessFilter,
}

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
struct Locations {
    exclude_locations: Vec<String>,
    include_locations: Vec<String>,
}

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
struct Platforms {
    exclude_platforms: Vec<DevicePlatforms>,
    include_platforms: Vec<DevicePlatforms>,
}

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
enum RiskLevel {
    Low,
    Medium,
    High,
    None,
    #[default]
    UnknownFutureValue,
}

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
enum InsiderRiskLevels {
    Minor,
    Moderate,
    Elevated,
    #[default]
    UnknownFutureValue,
}

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
enum DevicePlatforms {
    Android,
    #[serde(rename = "iOS")]
    Ios,
    Windows,
    WindowsPhone,
    #[serde(rename = "macOS")]
    MacOs,
    Linux,
    All,
    #[default]
    UnknownFutureValue,
}

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct ConditionalAccessPolicyConditionsUsers {
    exclude_groups: HashSet<String>,
    exclude_guests_or_external_users: Option<GuestOrExternalUsers>,
    exclude_roles: HashSet<String>,
    exclude_users: HashSet<String>,
    include_groups: HashSet<String>,
    include_guests_or_external_users: Option<GuestOrExternalUsers>,
    include_roles: HashSet<String>,
    include_users: HashSet<String>,
}

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
struct GuestOrExternalUsers {
    external_tenants: ExternalTenants,
    #[serde(deserialize_with = "deserialize_guest_or_external_user_types")]
    guest_or_external_user_types: Vec<GuestOrExternalUserTypes>,
}

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
struct ExternalTenants {
    #[serde(rename = "@odata.type")]
    odata_type: String,
    membership_kind: String,
}

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
enum GuestOrExternalUserTypes {
    None,
    InternalGuest,
    B2bCollaborationGuest,
    B2bCollaborationMember,
    B2bDirectConnectUser,
    OtherExternalUser,
    ServiceProvider,
    #[default]
    UnknownFutureValue,
}

fn deserialize_guest_or_external_user_types<'de, D>(
    deserializer: D,
) -> Result<Vec<GuestOrExternalUserTypes>, D::Error>
where
    D: Deserializer<'de>,
{
    let buf = String::deserialize(deserializer)?;
    let mut vec = vec![];
    for s in buf.split(",") {
        let guest_or_external_user_type = match s {
            "none" => GuestOrExternalUserTypes::None,
            "internalGuest" => GuestOrExternalUserTypes::InternalGuest,
            "b2bCollaborationGuest" => GuestOrExternalUserTypes::B2bCollaborationGuest,
            "b2bCollaborationMember" => GuestOrExternalUserTypes::B2bCollaborationMember,
            "b2bDirectConnectUser" => GuestOrExternalUserTypes::B2bDirectConnectUser,
            "otherExternalUser" => GuestOrExternalUserTypes::OtherExternalUser,
            "serviceProvider" => GuestOrExternalUserTypes::ServiceProvider,
            "unknownFutureValue" => GuestOrExternalUserTypes::UnknownFutureValue,
            _ => {
                return Err(serde::de::Error::custom(format!(
                    "Unknown GuestOrExternalUserType: {}",
                    s
                )));
            }
        };
        vec.push(guest_or_external_user_type);
    }
    Ok(vec)
}

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ConditionalAccessPolicies {
    #[serde(rename = "value")]
    pub inner: Vec<ConditionalAccessPolicy>,
}

impl ToHecEvents for &ConditionalAccessPolicies {
    type Item = ConditionalAccessPolicy;
    fn source(&self) -> &str {
        "msgraph"
    }

    fn sourcetype(&self) -> &str {
        "SSPHP.AAD.conditional_access_policy"
    }
    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        Box::new(self.inner.iter())
    }
    fn ssphp_run_key(&self) -> &str {
        "azure_resource_graph"
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

impl UserConditionalAccessPolicy<'_> {}

impl ConditionalAccessPolicies {
    pub fn new() -> Self {
        Self { inner: Vec::new() }
    }
}

impl IntoIterator for ConditionalAccessPolicies {
    type Item = ConditionalAccessPolicy;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.into_iter()
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
                visibility: None,
            }),
            GroupOrRole::Role(DirectoryRole {
                id: "role1".to_owned(),
                display_name: Some("role_1_name".to_owned()),
                role_template_id: "role1_template_id".to_owned(),
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
        _ = cap
            .conditions
            .users
            .include_users
            .insert(user.id.to_owned());
        assert!(cap.affects_user(&user));
    }

    #[test]
    fn affected_user_by_user_all() {
        let (user, mut cap) = setup();
        _ = cap.conditions.users.include_users.insert("All".to_owned());
        assert!(cap.affects_user(&user));
    }

    #[test]
    fn affected_user_by_group_all() {
        let (user, mut cap) = setup();
        _ = cap.conditions.users.include_groups.insert("All".to_owned());
        assert!(cap.affects_user(&user));
    }
    #[test]
    fn affected_user_by_group_id() {
        let (user, mut cap) = setup();
        _ = cap
            .conditions
            .users
            .include_groups
            .insert("group1".to_owned());
        assert!(cap.affects_user(&user));
    }
    #[test]
    fn affected_user_by_role_all() {
        let (user, mut cap) = setup();
        _ = cap.conditions.users.include_roles.insert("All".to_owned());
        assert!(cap.affects_user(&user));
    }
    #[test]
    fn affected_user_by_role_id() {
        let (user, mut cap) = setup();
        _ = cap
            .conditions
            .users
            .include_roles
            .insert("role1".to_owned());
        assert!(cap.affects_user(&user));
    }

    #[test]
    fn affected_user_by_role_template_id() {
        let (user, mut cap) = setup();
        _ = cap
            .conditions
            .users
            .include_roles
            .insert("role1_template_id".to_owned());
        assert!(cap.affects_user(&user));
    }

    #[test]
    fn affected_user_excluded_by_user_id() {
        let (user, mut cap) = setup();
        _ = cap
            .conditions
            .users
            .include_users
            .insert(user.id.to_owned());
        _ = cap
            .conditions
            .users
            .exclude_users
            .insert(user.id.to_owned());
        assert!(!cap.affects_user(&user));
    }

    #[test]
    fn affected_user_excluded_by_user_all() {
        let (user, mut cap) = setup();
        _ = cap
            .conditions
            .users
            .include_users
            .insert(user.id.to_owned());
        _ = cap.conditions.users.exclude_users.insert("All".to_owned());
        assert!(!cap.affects_user(&user));
    }

    #[test]
    fn affected_user_excluded_by_group_all() {
        let (user, mut cap) = setup();
        _ = cap
            .conditions
            .users
            .include_users
            .insert(user.id.to_owned());
        _ = cap.conditions.users.exclude_groups.insert("All".to_owned());
        assert!(!cap.affects_user(&user));
    }
    #[test]
    fn affected_user_excluded_by_group_id() {
        let (user, mut cap) = setup();
        _ = cap
            .conditions
            .users
            .include_users
            .insert(user.id.to_owned());
        _ = cap
            .conditions
            .users
            .exclude_groups
            .insert("group1".to_owned());
        assert!(!cap.affects_user(&user));
    }
    #[test]
    fn affected_user_excluded_by_role_all() {
        let (user, mut cap) = setup();
        _ = cap
            .conditions
            .users
            .include_users
            .insert(user.id.to_owned());
        _ = cap.conditions.users.exclude_roles.insert("All".to_owned());
        assert!(!cap.affects_user(&user));
    }
    #[test]
    fn affected_user_excluded_by_role_id() {
        let (user, mut cap) = setup();
        _ = cap
            .conditions
            .users
            .include_users
            .insert(user.id.to_owned());
        _ = cap
            .conditions
            .users
            .exclude_roles
            .insert("role1".to_owned());
        assert!(!cap.affects_user(&user));
    }

    #[test]
    fn affected_user_excluded_by_role_template_id() {
        let (user, mut cap) = setup();
        _ = cap
            .conditions
            .users
            .include_users
            .insert(user.id.to_owned());
        _ = cap
            .conditions
            .users
            .exclude_roles
            .insert("role1_template_id".to_owned());
        assert!(!cap.affects_user(&user));
    }
}
