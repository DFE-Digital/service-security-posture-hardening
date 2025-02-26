use crate::admin_request_consent_policy::AdminRequestConsentPolicy;

use crate::conditional_access_policies::ConditionalAccessPolicies;
use crate::groups::Groups;
use data_ingester_supporting::dns::resolve_txt_record;
use data_ingester_supporting::keyvault::Secrets;
use graph_oauth::ClientSecretCredential;
use graph_rs_sdk::GraphClient;
use graph_rs_sdk::GraphClientConfiguration;
use tracing::info;

use crate::msgraph_data::load_m365_toml;
use crate::roles::RoleDefinitions;
use crate::users::Users;
use crate::users::UsersMap;
use anyhow::{Context, Result};
use data_ingester_splunk::splunk::try_collect_send;
use data_ingester_splunk::splunk::ToHecEvents;
use data_ingester_splunk::splunk::{set_ssphp_run, Splunk};
use futures::StreamExt;
use graph_http::api_impl::RequestComponents;
use graph_http::api_impl::RequestHandler;
use graph_oauth::ConfidentialClientApplication;
use graph_rs_sdk::header::HeaderMap;
use graph_rs_sdk::header::HeaderValue;
use graph_rs_sdk::header::CONTENT_TYPE;
use graph_rs_sdk::http::Method;
use graph_rs_sdk::Graph;
use graph_rs_sdk::ODataQuery;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;
use std::env;
use std::fmt::Debug;
use std::iter;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc::UnboundedSender;

/// A Client for Ms Graph
#[derive(Clone)]
pub struct MsGraph {
    client: Graph,
    beta_client: Graph,
    client_application: ConfidentialClientApplication<ClientSecretCredential>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(untagged)]
pub enum MsGraphGetResponse {
    Collection { value: Vec<serde_json::Value> },
    Single(serde_json::Value),
}

impl MsGraph {
    pub async fn new(client_id: &str, client_secret: &str, tenant_id: &str) -> Result<MsGraph> {
        let client_application = ConfidentialClientApplication::builder(client_id)
            .with_client_secret(client_secret)
            .with_tenant(tenant_id)
            .build();

        let client_config = GraphClientConfiguration::new()
            .client_application(client_application.clone())
            .timeout(Duration::from_secs(30))
            .retry(Some(10))
            .wait_for_retry_after_headers(true);

        let client = GraphClient::from(client_config.clone());

        let mut beta_client = GraphClient::from(client_config.clone());
        beta_client.use_beta();

        Ok(MsGraph {
            client,
            beta_client,
            client_application,
        })
    }

    pub async fn get_url(&self, url: &str) -> Result<Vec<Value>> {
        let current_client = graph_http::api_impl::Client::builder()
            .client_application(self.client_application.clone())
            .retry(Some(20))
            .wait_for_retry_after_headers(true)
            .build();

        let mut header_map = HeaderMap::new();

        _ = header_map
            .entry(CONTENT_TYPE)
            .or_insert(HeaderValue::from_static("application/json"));

        let full_url = self.client.url().join(url)?;

        let request_components = RequestComponents::new(
            graph_core::resource::ResourceIdentity::Custom,
            full_url,
            Method::GET,
        );

        let request_handler =
            RequestHandler::new(current_client.clone(), request_components, None, None)
                .headers(header_map);

        let mut stream = request_handler.paging().stream::<MsGraphGetResponse>()?;

        let mut collection = Vec::default();
        while let Some(result) = stream.next().await {
            let response = result?;
            let body = response.into_body()?;

            match body {
                MsGraphGetResponse::Collection { value } => collection.extend(value),
                MsGraphGetResponse::Single(value) => collection.push(value),
            }
        }
        Ok(collection)
    }

    /// 2.10
    /// MSGraph Permission: OrgSettings-Forms.Read.All
    /// https://graph.microsoft.com/beta/admin/forms
    pub async fn get_admin_form_settings(&self) -> Result<AdminFormSettings> {
        let result = self.get_url("/beta/admin/forms").await?;
        Ok(AdminFormSettings { inner: result })
    }

    /// 1.1.9
    /// 1.1.10
    /// https://learn.microsoft.com/en-us/graph/api/resources/groupsetting?view=graph-rest-1.0
    /// The /beta version of this resource is named directorySetting.
    pub async fn list_group_settings(&self) -> Result<GroupSettings> {
        let result = self.get_url("/groupSettings").await?;
        Ok(GroupSettings { inner: result })
    }

    pub async fn list_role_eligiblity_schedule_instance(
        &self,
    ) -> Result<RoleEligibilityScheduleInstance> {
        let result = self.get_url("/roleManagement/directory/roleAssignmentScheduleInstances?$expand=activatedUsing,appScope,directoryScope,principal,roleDefinition").await?;
        Ok(RoleEligibilityScheduleInstance { inner: result })
    }

    /// M365 V2 1.1.17
    pub async fn list_legacy_policies(&self) -> Result<LegacyPolicies> {
        let result = self.get_url("/legacy/policies").await?;
        Ok(LegacyPolicies { inner: result })
    }

    // /// M365 V2 1.1.18
    // /// This does not work - No such API
    // pub async fn get_app_family_details(&self) -> Result<AppFamilyDetails> {
    //     let result = self
    //         .get(&self.beta_client, "/organization/getAppFamilyDetails")
    //         .await?;
    //     Ok(AppFamilyDetails { inner: result })
    // }

    pub async fn list_conditional_access_policies(&self) -> Result<ConditionalAccessPolicies> {
        let mut stream = self
            .client
            .identity()
            .list_policies()
            .paging()
            .stream::<ConditionalAccessPolicies>()?;

        let mut caps = ConditionalAccessPolicies::new();
        while let Some(result) = stream.next().await {
            let response = result?;

            let body = response.into_body();

            caps.inner.extend(body?.inner)
        }
        Ok(caps)
    }

    /// Azure 1.2.1
    /// https://learn.microsoft.com/en-us/graph/api/conditionalaccessroot-list-namedlocations?view=graph-rest-1.0&tabs=http
    pub async fn list_named_locations(&self) -> Result<NamedLocations> {
        let mut stream = self
            .client
            .identity()
            .list_named_locations()
            .paging()
            .stream::<NamedLocations>()?;

        let mut collection = NamedLocations::default();
        while let Some(result) = stream.next().await {
            let response = result?;

            let body = response.into_body();

            collection.inner.extend(body?.inner)
        }
        Ok(collection)
    }

    pub async fn list_role_definitions(&self) -> Result<RoleDefinitions> {
        let mut stream = self
            .beta_client
            .custom(Method::GET, None)
            .extend_path(&["roleManagement", "directory", "roleDefinitions"])
            .paging()
            .stream::<RoleDefinitions>()?;

        let mut roles = RoleDefinitions::new();
        while let Some(result) = stream.next().await {
            let response = result?;

            let body = response.into_body()?;

            roles.value.extend(body.value)
        }
        Ok(roles)
    }

    pub async fn list_groups(&self) -> Result<Groups> {
        let mut stream = self
            .client
            .groups()
            .list_group()
            .paging()
            .stream::<Groups>()?;

        let mut groups = Groups::default();

        while let Some(result) = stream.next().await {
            let body = result?.into_body()?;
            groups.inner.extend(body.inner);
        }

        Ok(groups)
    }

    pub async fn list_users_channel(&self, sender: UnboundedSender<UsersMap<'_>>) -> Result<()> {
        let mut stream = self
            .beta_client
            .users()
            .list_user()
            .select(&[
                "accountEnabled",
                "assignedPlans",
                "description",
                "displayName",
                "givenName",
                "id",
                "mail",
                "onPremisesSamAccountName",
                "onPremisesSyncEnabled",
                "surname",
                "transitiveMemberOf",
                "userPrincipalName",
                "userType",
                // TODO Needs AuditLog.Read.All MS Graph permission
                // "signInActivity",
            ])
            .expand(&["transitiveMemberOf"])
            .top("999")
            .paging()
            .stream::<Users>()?;

        let mut total_users = 0;
        let mut batch = 1;

        while let Some(result) = stream.next().await {
            let response = result?;

            let mut users = response.into_body()?;

            total_users += users.value.len();

            info!(
                "Getting users batch {}, total users: {}",
                batch, total_users
            );
            batch += 1;

            users.value.iter_mut().for_each(|u| {
                u.assigned_plans_remove_deleted();
            });

            let mut users_map = UsersMap::default();

            users_map
                .extend_from_users(users)
                .context("unable to extend users")?;

            sender
                .send(users_map)
                .expect("Unable to send Users to channel");
        }

        Ok(())
    }

    pub async fn get_admin_request_consent_policy(&self) -> Result<AdminRequestConsentPolicy> {
        let response = self
            .client
            .policies()
            .get_admin_consent_request_policy()
            .send()
            .await?;
        let body = response.json::<AdminRequestConsentPolicy>().await?;
        Ok(body)
    }

    pub async fn get_authentication_methods_policy(&self) -> Result<AuthenticationMethodsPolicy> {
        let response = self
            .client
            .policies()
            .get_authentication_methods_policy()
            .send()
            .await?;
        let body = response.json().await?;
        Ok(body)
    }

    pub async fn get_domains(&self) -> Result<Domains> {
        let response = self.client.domains().list_domain().send().await?;
        let mut body: Domains = response.json().await?;
        for domain in body.inner.iter_mut() {
            if let Ok(txt) = resolve_txt_record(&domain.id).await {
                domain.txt_records = Some(txt);
            }

            let dmarc_domain = format!("_dmarc.{}", &domain.id);
            if let Ok(dmarc) = resolve_txt_record(&dmarc_domain).await {
                domain.dmarc = Some(dmarc);
            }
        }
        Ok(body)
    }

    pub async fn get_authorization_policy(&self) -> Result<AuthorizationPolicy> {
        let response = self
            .beta_client
            .policies()
            .get_authorization_policy()
            .send()
            .await?;
        let body = response.json().await?;
        Ok(body)
    }

    /// MS Graph Permission Policy.Read.PermissionGrant
    pub async fn list_permission_grant_policy(&self) -> Result<PermissionGrantPolicy> {
        let response = self
            .client
            .policies()
            .list_permission_grant_policies()
            .send()
            .await?;
        let body = response.json().await?;
        Ok(body)
    }

    // M365 1.1.1
    // Azure 1.1.1
    pub async fn get_identity_security_defaults_enforcement_policy(
        &self,
    ) -> Result<IdentitySecurityDefaultsEnforcementPolicy> {
        let response = self
            .client
            .policies()
            .get_identity_security_defaults_enforcement_policy()
            .send()
            .await?;
        let body = response.json().await?;
        Ok(body)
    }

    #[allow(dead_code)]
    pub async fn list_token_lifetime_policies(&self) -> Result<Value> {
        let response = self
            .client
            .policies()
            .list_token_lifetime_policies()
            .send()
            .await?;
        let body = response.json().await?;
        Ok(body)
    }

    /// 5.1.1
    /// Permission: AccessReview.Read.All
    pub async fn get_access_reviews(&self) -> Result<AccessReviewDefinitions> {
        let response = self
            .client
            .identity_governance()
            .access_reviews()
            .definitions()
            .list_definitions()
            .send()
            .await?;
        let body = response.json().await?;
        Ok(body)
    }

    /// 1.22
    pub async fn get_device_registration_policy(&self) -> Result<DeviceRegistrationPolicy> {
        let result = self.get_url("/policies/deviceRegistrationPolicy").await?;
        Ok(DeviceRegistrationPolicy { inner: result })
    }
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct LegacyPolicies {
    #[serde(rename = "value")]
    inner: Vec<serde_json::Value>,
}

impl ToHecEvents for &LegacyPolicies {
    type Item = Value;
    fn source(&self) -> &str {
        "msgraph"
    }

    fn sourcetype(&self) -> &str {
        "msgraph:legacy_policy"
    }

    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        Box::new(self.inner.iter())
    }
    fn ssphp_run_key(&self) -> &str {
        "m365"
    }
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct DeviceRegistrationPolicy {
    inner: Vec<Value>,
}

impl ToHecEvents for &DeviceRegistrationPolicy {
    type Item = Value;
    fn source(&self) -> &str {
        "msgraph"
    }

    fn sourcetype(&self) -> &str {
        "msgraph:device_registration_policy"
    }

    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        Box::new(self.inner.iter())
    }
    fn ssphp_run_key(&self) -> &str {
        "m365"
    }
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct NamedLocations {
    #[serde(rename = "value")]
    inner: Vec<serde_json::Value>,
}

impl ToHecEvents for &NamedLocations {
    type Item = Value;
    fn source(&self) -> &str {
        "msgraph"
    }

    fn sourcetype(&self) -> &str {
        "identity/named_locations"
    }

    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        Box::new(self.inner.iter())
    }
    fn ssphp_run_key(&self) -> &str {
        "m365"
    }
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct AccessReviewDefinitions {
    #[serde(rename = "value")]
    inner: Vec<serde_json::Value>,
}

impl ToHecEvents for &AccessReviewDefinitions {
    type Item = Value;
    fn source(&self) -> &str {
        "msgraph"
    }

    fn sourcetype(&self) -> &str {
        "m365:access_review_definitions"
    }

    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        Box::new(self.inner.iter())
    }
    fn ssphp_run_key(&self) -> &str {
        "m365"
    }
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct AdminFormSettings {
    #[serde(rename = "settings")]
    inner: Vec<serde_json::Value>,
}

impl ToHecEvents for &AdminFormSettings {
    type Item = Value;
    fn source(&self) -> &str {
        "msgraph"
    }

    fn sourcetype(&self) -> &str {
        "m365:admin_form_settings"
    }

    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        Box::new(self.inner.iter())
    }
    fn ssphp_run_key(&self) -> &str {
        "m365"
    }
}

// #[derive(Debug, Serialize, Deserialize, Default)]
// pub struct AppFamilyDetails {
//     #[serde(flatten)]
//     inner: Vec<Value>,
// }

// impl ToHecEvents for &AppFamilyDetails {
//     type Item = Value;
//     fn source(&self) -> &str {
//         "msgraph"
//     }

//     fn sourcetype(&self) -> &str {
//         "m365:app_family_details"
//     }

//     fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
//         Box::new(self.inner.iter())
//     }
// }

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct AuthorizationPolicy {
    #[serde(rename = "@odata.context")]
    pub odata_context: String,
    #[serde(flatten)]
    value: serde_json::Value,
}

impl ToHecEvents for &AuthorizationPolicy {
    type Item = Self;
    fn source(&self) -> &str {
        "msgraph"
    }

    fn sourcetype(&self) -> &str {
        "m365:authorization_policy"
    }

    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        Box::new(iter::once(self))
    }
    fn ssphp_run_key(&self) -> &str {
        "m365"
    }
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct RoleEligibilityScheduleInstance {
    #[serde(rename = "value")]
    inner: Vec<serde_json::Value>,
}

impl ToHecEvents for &RoleEligibilityScheduleInstance {
    type Item = Value;
    fn source(&self) -> &str {
        "msgraph"
    }

    fn sourcetype(&self) -> &str {
        "m365:role_eligibility_schedule_instance"
    }

    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        Box::new(self.inner.iter())
    }

    fn ssphp_run_key(&self) -> &str {
        "m365"
    }
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct GroupSettings {
    #[serde(rename = "value")]
    inner: Vec<serde_json::Value>,
}

impl ToHecEvents for &GroupSettings {
    type Item = Value;
    fn source(&self) -> &str {
        "msgraph"
    }

    fn sourcetype(&self) -> &str {
        "m365:group_settings"
    }

    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        Box::new(self.inner.iter())
    }
    fn ssphp_run_key(&self) -> &str {
        "m365"
    }
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct AuthenticationMethodsPolicy(serde_json::Value);

impl ToHecEvents for &AuthenticationMethodsPolicy {
    type Item = Self;
    fn source(&self) -> &str {
        "msgraph"
    }

    fn sourcetype(&self) -> &str {
        "m365:authentication_methods_policy"
    }

    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        Box::new(iter::once(self))
    }
    fn ssphp_run_key(&self) -> &str {
        "m365"
    }
}

/// CIS Azure 365 Azure 1.4
/// CIS Azure 365 Azure 4.8
/// CIS Azure 365 Azure 4.9
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Domains {
    #[serde(rename = "@odata.context")]
    pub odata_context: String,
    #[serde(rename = "value")]
    pub inner: Vec<Domain>,
}

/// CIS Azure 365 Azure 1.4
/// CIS Azure 365 Azure 4.8
/// CIS Azure 365 Azure 4.9
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Domain {
    pub authentication_type: String,
    pub availability_status: Option<String>,
    pub id: String,
    pub is_admin_managed: bool,
    pub is_default: bool,
    pub is_initial: bool,
    pub is_root: bool,
    pub is_verified: bool,
    pub password_notification_window_in_days: Option<i32>,
    pub password_validity_period_in_days: Option<i32>,
    pub state: Value,
    pub supported_services: Vec<String>,
    pub txt_records: Option<Vec<String>>,
    pub dmarc: Option<Vec<String>>,
}

impl ToHecEvents for &Domains {
    type Item = Domain;
    fn source(&self) -> &str {
        "/v1.0/domains"
    }

    fn sourcetype(&self) -> &str {
        "ssphp:ms_graph:json"
    }

    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        Box::new(self.inner.iter())
    }
    fn ssphp_run_key(&self) -> &str {
        "m365"
    }
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct PermissionGrantPolicy {
    #[serde(rename = "value")]
    inner: Vec<serde_json::Value>,
}

impl ToHecEvents for &PermissionGrantPolicy {
    type Item = Value;
    fn source(&self) -> &'static str {
        "msgraph"
    }

    fn sourcetype(&self) -> &'static str {
        "m365:permission_grant_policy"
    }

    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        Box::new(self.inner.iter())
    }

    fn ssphp_run_key(&self) -> &str {
        "m365"
    }
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct IdentitySecurityDefaultsEnforcementPolicy(serde_json::Value);

impl ToHecEvents for &IdentitySecurityDefaultsEnforcementPolicy {
    type Item = Self;
    fn source(&self) -> &'static str {
        "msgraph"
    }

    fn sourcetype(&self) -> &'static str {
        "m365:identitySecurityDefaultsEnforcementPolicy"
    }

    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        Box::new(iter::once(self))
    }

    fn ssphp_run_key(&self) -> &str {
        "m365"
    }
}

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Group {
    pub id: String,
    // pub deleted_date_time: Value,
    // pub classification: Value,
    // pub created_date_time: String,
    // pub creation_options: Vec<Value>,
    // pub description: Value,
    pub display_name: Option<String>,
    // pub expiration_date_time: Value,
    // pub group_types: Vec<String>,
    // pub is_assignable_to_role: Value,
    // pub mail: String,
    // pub mail_enabled: bool,
    // pub mail_nickname: String,
    // pub membership_rule: Value,
    // pub membership_rule_processing_state: Value,
    // pub on_premises_domain_name: Value,
    // pub on_premises_last_sync_date_time: Value,
    // pub on_premises_net_bios_name: Value,
    // pub on_premises_sam_account_name: Value,
    // pub on_premises_security_identifier: Value,
    // pub on_premises_sync_enabled: Value,
    // pub preferred_data_location: Value,
    // pub preferred_language: Value,
    // pub proxy_addresses: Vec<String>,
    // pub renewed_date_time: String,
    // pub resource_behavior_options: Vec<Value>,
    // pub resource_provisioning_options: Vec<Value>,
    pub security_enabled: Option<bool>,
    pub security_identifier: Option<String>,
    // pub theme: Value,3
    pub visibility: Option<String>,
    // pub on_premises_provisioning_errors: Vec<Value>,
    // pub service_provisioning_errors: Vec<Value>,
}

pub async fn m365(secrets: Arc<Secrets>, splunk: Arc<Splunk>) -> Result<()> {
    let ssphp_run_key = "m365";
    set_ssphp_run(ssphp_run_key)?;

    info!("Starting M365 collection");
    info!("GIT_HASH: {}", env!("GIT_HASH"));

    let ms_graph = MsGraph::new(
        secrets
            .azure_client_id
            .as_ref()
            .context("Expect azure_client_id secret")?,
        secrets
            .azure_client_secret
            .as_ref()
            .context("Expect azure_client_secret secret")?,
        secrets
            .azure_tenant_id
            .as_ref()
            .context("Expect azure_tenant_id secret")?,
    )
    .await?;

    info!("MS Graph logged in");

    let sources = load_m365_toml()?;

    info!("Loaded {} m365 sources", sources.len());
    sources
        .process_sources(&ms_graph, &splunk, ssphp_run_key)
        .await?;

    // M365 1.1.17 V2
    let _ = try_collect_send(
        "MS Graph List Role Eligibility Schedules",
        ms_graph.list_legacy_policies(),
        &splunk,
    )
    .await;

    // M365 1.1.15 V2
    let _ = try_collect_send(
        "MS Graph List Role Eligibility Schedules",
        ms_graph.list_role_eligiblity_schedule_instance(),
        &splunk,
    )
    .await;

    // Azure Foundations 1.22 V2
    let _ = try_collect_send(
        "MS Graph Device Registration Policy",
        ms_graph.get_device_registration_policy(),
        &splunk,
    )
    .await;

    // 5.1.1
    let _ = try_collect_send(
        "MS Graph Access Reviews",
        ms_graph.get_access_reviews(),
        &splunk,
    )
    .await;

    // 1.1.9
    // 1.1.10
    let _ = try_collect_send(
        "MS Graph Admin form settings",
        ms_graph.get_admin_form_settings(),
        &splunk,
    )
    .await;

    // 1.1.9
    // 1.1.10
    let _ = try_collect_send(
        "MS Graph Group Settings",
        ms_graph.list_group_settings(),
        &splunk,
    )
    .await;

    // 1.2.1
    let _ = try_collect_send(
        "MS Graph Named Locations",
        ms_graph.list_named_locations(),
        &splunk,
    )
    .await;

    let _ = try_collect_send(
        "MS Graph Authentication Methods Policy",
        ms_graph.get_authentication_methods_policy(),
        &splunk,
    )
    .await;

    let _ = try_collect_send(
        "MS Graph Authorization Policy",
        ms_graph.get_authorization_policy(),
        &splunk,
    )
    .await;

    let _ = try_collect_send(
        "MS Graph Admin RequestConsent Policy",
        ms_graph.get_admin_request_consent_policy(),
        &splunk,
    )
    .await;

    let _ = try_collect_send("MS Graph Domains", ms_graph.get_domains(), &splunk).await;

    let _ = try_collect_send(
        "MS Graph Permission Grant Policy",
        ms_graph.list_permission_grant_policy(),
        &splunk,
    )
    .await;

    let _ = try_collect_send(
        "Exchange Get Security Default Policy",
        ms_graph.get_identity_security_defaults_enforcement_policy(),
        &splunk,
    )
    .await;

    let _ = try_collect_send("MS Graph Groups", ms_graph.list_groups(), &splunk).await;

    info!("M365 Collection Complete");

    Ok(())
}

#[cfg(feature = "live_tests")]
#[cfg(test)]
pub(crate) mod live_tests {
    use std::env;

    use super::MsGraph;
    use crate::users::UsersMap;

    use anyhow::{Context, Result};
    use data_ingester_splunk::splunk::{set_ssphp_run, Splunk, ToHecEvents};
    use data_ingester_supporting::keyvault::get_keyvault_secrets;

    pub async fn setup() -> Result<(Splunk, MsGraph)> {
        let secrets = get_keyvault_secrets(&env::var("KEY_VAULT_NAME")?).await?;

        set_ssphp_run("default")?;
        let ms_graph = MsGraph::new(
            secrets
                .azure_client_id
                .as_ref()
                .context("Expect azure_client_id secret")?,
            secrets
                .azure_client_secret
                .as_ref()
                .context("Expect azure_client_secret secret")?,
            secrets
                .azure_tenant_id
                .as_ref()
                .context("Expect azure_tenant_id secret")?,
        )
        .await?;

        let splunk = Splunk::new(
            secrets.splunk_host.as_ref().context("No value")?,
            secrets.splunk_token.as_ref().context("No value")?,
            true,
        )?;

        Ok((splunk, ms_graph))
    }

    #[tokio::test]
    async fn get_users_channel() -> Result<()> {
        let (splunk, ms_graph) = setup().await?;

        let (sender, mut reciever) = tokio::sync::mpsc::unbounded_channel::<UsersMap>();

        let ms_graph_clone = ms_graph.clone();
        let list_users = tokio::spawn(async move {
            ms_graph_clone.list_users_channel(sender).await?;
            anyhow::Ok::<()>(())
        });

        let mut users_map = UsersMap::default();
        while let Some(users) = reciever.recv().await {
            users_map.extend(users);
        }
        let _ = list_users.await?;

        assert!(!users_map.inner.is_empty());
        splunk.send_batch((&users_map).to_hec_events()?).await?;
        Ok(())
    }

    #[tokio::test]
    async fn get_admin_request_consent_policy() -> Result<()> {
        let (splunk, ms_graph) = setup().await?;
        let admin_request_consent_policy = ms_graph.get_admin_request_consent_policy().await?;
        splunk
            .send_batch((&admin_request_consent_policy).to_hec_events()?)
            .await?;
        Ok(())
    }

    #[tokio::test]
    async fn get_authorization_policy() -> Result<()> {
        let (splunk, ms_graph) = setup().await?;
        let get_authorization_policy = ms_graph.get_authorization_policy().await?;
        splunk
            .send_batch((&get_authorization_policy).to_hec_events()?)
            .await?;
        Ok(())
    }

    #[tokio::test]
    async fn authentication_methods_policy() -> Result<()> {
        let (splunk, ms_graph) = setup().await?;
        let authentication_methods_policy = ms_graph.get_authentication_methods_policy().await?;
        splunk
            .send_batch((&authentication_methods_policy).to_hec_events()?)
            .await?;
        Ok(())
    }

    #[tokio::test]
    async fn list_conditional_access_policies() -> Result<()> {
        let (splunk, ms_graph) = setup().await?;
        let caps = ms_graph.list_conditional_access_policies().await?;
        splunk.send_batch((&caps).to_hec_events()?).await?;
        Ok(())
    }

    #[tokio::test]
    async fn get_domains() -> Result<()> {
        let (splunk, ms_graph) = setup().await?;
        let domains = ms_graph.get_domains().await?;
        splunk.send_batch((&domains).to_hec_events()?).await?;
        Ok(())
    }

    #[tokio::test]
    async fn list_groups() -> Result<()> {
        let (splunk, ms_graph) = setup().await?;
        let groups = ms_graph.list_groups().await?;
        splunk.send_batch((&groups).to_hec_events()?).await?;
        Ok(())
    }

    #[tokio::test]
    async fn get_permission_grant_policy() -> Result<()> {
        let (splunk, ms_graph) = setup().await?;
        let permission_grant_policy = ms_graph.list_permission_grant_policy().await?;
        splunk
            .send_batch((&permission_grant_policy).to_hec_events()?)
            .await?;
        Ok(())
    }

    #[tokio::test]
    async fn get_identity_security_defaults_enforcement_policy() -> Result<()> {
        let (splunk, ms_graph) = setup().await?;
        let security_defaults = ms_graph
            .get_identity_security_defaults_enforcement_policy()
            .await?;
        splunk
            .send_batch((&security_defaults).to_hec_events()?)
            .await?;
        Ok(())
    }

    #[tokio::test]
    async fn list_group_settings() -> Result<()> {
        let (splunk, ms_graph) = setup().await?;
        let result = ms_graph.list_group_settings().await?;
        splunk.send_batch((&result).to_hec_events()?).await?;
        Ok(())
    }

    #[tokio::test]
    async fn get_admin_forms() -> Result<()> {
        let (splunk, ms_graph) = setup().await?;
        let result = ms_graph.get_admin_form_settings().await?;
        splunk.send_batch((&result).to_hec_events()?).await?;
        Ok(())
    }

    #[tokio::test]
    async fn get_access_reviews() -> Result<()> {
        let (splunk, ms_graph) = setup().await?;
        let result = ms_graph.get_access_reviews().await?;
        splunk.send_batch((&result).to_hec_events()?).await?;
        Ok(())
    }

    #[tokio::test]
    async fn list_named_locations() -> Result<()> {
        let (splunk, ms_graph) = setup().await?;
        let result = ms_graph.list_named_locations().await?;
        splunk.send_batch((&result).to_hec_events()?).await?;
        Ok(())
    }

    // #[ignore]
    // #[tokio::test]
    // async fn list_token_lifetime_policies() -> Result<()> {
    //     let (splunk, ms_graph) = setup().await?;
    //     let result = ms_graph.list_token_lifetime_policies().await?;
    //     let hec = HecDynamic::new(result, "msgraph:tokenlifetime", "aktest");
    //     splunk.send_batch((&hec).to_hec_events()?).await?;
    //     Ok(())
    // }

    #[tokio::test]
    async fn list_permission_grant_policies() -> Result<()> {
        let (splunk, ms_graph) = setup().await?;
        let result = ms_graph.list_permission_grant_policy().await?;
        splunk.send_batch((&result).to_hec_events()?).await?;
        Ok(())
    }

    #[tokio::test]
    async fn get_device_registration_policy() -> Result<()> {
        let (splunk, ms_graph) = setup().await?;
        let result = ms_graph.get_device_registration_policy().await?;
        splunk.send_batch((&result).to_hec_events()?).await?;
        Ok(())
    }

    #[tokio::test]
    async fn list_role_eligiblity_schedule_instance() -> Result<()> {
        let (splunk, ms_graph) = setup().await?;
        let result = ms_graph.list_role_eligiblity_schedule_instance().await?;
        splunk.send_batch((&result).to_hec_events()?).await?;
        Ok(())
    }

    #[tokio::test]
    async fn list_legacy_policies() -> Result<()> {
        let (splunk, ms_graph) = setup().await?;
        let result = ms_graph.list_legacy_policies().await?;
        splunk.send_batch((&result).to_hec_events()?).await?;
        Ok(())
    }

    #[tokio::test]
    async fn list_role_definitions() -> Result<()> {
        let (splunk, ms_graph) = setup().await?;
        let result = ms_graph.list_role_definitions().await?;
        splunk.send_batch((&result).to_hec_events()?).await?;
        Ok(())
    }

    // API only exists in an undocumented preview state
    // #[ignore]
    // #[tokio::test]
    // async fn get_app_family_details() -> Result<()> {
    //     let (splunk, ms_graph) = setup().await?;
    //     let result = ms_graph.get_app_family_details().await?;
    //     splunk.send_batch((&result).to_hec_events()?).await?;
    //     Ok(())
    // }
}
