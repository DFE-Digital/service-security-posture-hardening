use crate::admin_request_consent_policy::AdminRequestConsentPolicy;
use crate::azure_rest::AzureRest;
use crate::conditional_access_policies::ConditionalAccessPolicies;
use crate::dns::resolve_txt_record;
use crate::groups::Groups;
use crate::keyvault::Secrets;
use crate::powershell::run_powershell_get_admin_audit_log_config;
use crate::powershell::run_powershell_get_anti_phish_policy;
use crate::powershell::run_powershell_get_atp_policy_for_o365;
use crate::powershell::run_powershell_get_blocked_sender_address;
use crate::powershell::run_powershell_get_dkim_signing_config;
use crate::powershell::run_powershell_get_dlp_compliance_policy;
use crate::powershell::run_powershell_get_email_tenant_settings;
use crate::powershell::run_powershell_get_hosted_outbound_spam_filter_policy;
use crate::powershell::run_powershell_get_malware_filter_policy;
use crate::powershell::run_powershell_get_organization_config;
use crate::powershell::run_powershell_get_owa_mailbox_policy;
use crate::powershell::run_powershell_get_safe_attachment_policy;
use crate::powershell::run_powershell_get_safe_links_policy;
use crate::powershell::run_powershell_get_sharing_policy;
use crate::powershell::run_powershell_get_spoof_intelligence_insight;
use crate::powershell::run_powershell_get_transport_rule;
use crate::roles::RoleDefinitions;
use crate::security_score::SecurityScores;
use crate::splunk::HecEvent;
use crate::splunk::Message;
use crate::splunk::ToHecEvents;
use crate::splunk::{set_ssphp_run, Splunk};
use crate::users::Users;
use crate::users::UsersMap;
use anyhow::{Context, Result};
use futures::Future;
use futures::StreamExt;
use graph_rs_sdk::oauth::AccessToken;
use graph_rs_sdk::oauth::OAuth;
use graph_rs_sdk::Graph;
use graph_rs_sdk::ODataQuery;
use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;
use std::env;
use std::fmt::Debug;
use std::iter;
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedSender;

pub async fn login(client_id: &str, client_secret: &str, tenant_id: &str) -> Result<MsGraph> {
    let mut oauth = OAuth::new();
    oauth
        .client_id(client_id)
        .client_secret(client_secret)
        .add_scope("https://graph.microsoft.com/.default")
        .tenant_id(tenant_id);
    let mut request = oauth.build_async().client_credentials();
    let response = request.access_token().send().await?;

    if response.status().is_success() {
        let access_token: AccessToken = response.json().await?;

        oauth.access_token(access_token);
    } else {
        // See if Microsoft Graph returned an error in the Response body
        let result: reqwest::Result<serde_json::Value> = response.json().await;
        println!("{result:#?}");
    }

    let client = Graph::new(
        oauth
            .get_access_token()
            .context("no access token")?
            .bearer_token(),
    );
    let mut beta_client = Graph::new(
        oauth
            .get_access_token()
            .context("no access token")?
            .bearer_token(),
    );
    beta_client.use_beta();

    Ok(MsGraph {
        client,
        beta_client,
        oauth,
    })
}

#[derive(Clone)]
pub struct MsGraph {
    client: Graph,
    beta_client: Graph,
    oauth: OAuth,
}

impl MsGraph {
    async fn batch_get<T>(&self, client: &Graph, url: &str) -> Result<T>
    where
        T: DeserializeOwned + Debug + Default,
    {
        let json = serde_json::json!({
            "requests": [
                {
                    "id": "1",
                    "method": "GET",
                    "url": url,
                },
            ]
        });

        let response = client.batch(&json).send().await?;

        let mut batch_response: MsGraphBatch<T> = response.json().await?;

        let result = std::mem::take(
            &mut batch_response
                .responses
                .first_mut()
                .context("no response")?
                .body,
        );

        Ok(result)
    }

    /// 2.10
    /// MSGraph Permission: OrgSettings-Forms.Read.All
    /// https://graph.microsoft.com/beta/admin/forms
    pub async fn get_admin_form_settings(&self) -> Result<AdminFormSettings> {
        let client = reqwest::Client::new();
        let (client, request) = client
            .get("https://graph.microsoft.com/beta/admin/forms")
            .header(
                "Authorization",
                &format!(
                    "Bearer {}",
                    self.oauth
                        .get_access_token()
                        .context("can't get access token")?
                        .bearer_token()
                ),
            )
            .build_split();

        let response = client.execute(request?).await?;

        let body = response.json().await?;

        Ok(body)
    }

    /// 1.1.9
    /// 1.1.10
    /// https://learn.microsoft.com/en-us/graph/api/resources/groupsetting?view=graph-rest-1.0
    /// The /beta version of this resource is named directorySetting.
    pub async fn list_group_settings(&self) -> Result<GroupSettings> {
        let result = self
            .batch_get(&self.client, "/groupSettingTemplates")
            .await?;
        Ok(result)
    }

    pub async fn list_conditional_access_policies(&self) -> Result<ConditionalAccessPolicies> {
        let mut stream = self
            .client
            .identity()
            .list_policies()
            .paging()
            .stream::<ConditionalAccessPolicies>()?;

        let mut caps = ConditionalAccessPolicies::new();
        while let Some(result) = stream.next().await {
            let response = result.unwrap();

            let body = response.into_body();

            caps.inner.extend(body.unwrap().inner)
        }
        Ok(caps)
    }

    // pub async fn list_directory_roles(&self) -> DirectoryRoles {
    //     let mut stream = self
    //         .client
    //         .directory_roles()
    //         .list_directory_role()
    //         .expand(&["members"])
    //         .paging()
    //         .stream::<DirectoryRoles>()
    //         .unwrap();

    //     let mut directory_roles = DirectoryRoles::new();
    //     while let Some(result) = stream.next().await {
    //         let response = result.unwrap();

    //         let body = response.into_body();

    //         directory_roles.value.extend(body.unwrap().value)
    //     }
    //     directory_roles
    // }

    // pub async fn list_directory_role_templates(&self) -> DirectoryRoleTemplates {
    //     let mut stream = self
    //         .client
    //         .directory_role_templates()
    //         .list_directory_role_template()
    //         .paging()
    //         .stream::<DirectoryRoleTemplates>()
    //         .unwrap();

    //     let mut directory_role_templates = DirectoryRoleTemplates::new();
    //     while let Some(result) = stream.next().await {
    //         let response = result.unwrap();

    //         let body = response.into_body();

    //         directory_role_templates.value.extend(body.unwrap().value)
    //     }
    //     directory_role_templates
    // }

    pub async fn list_role_definitions(&self) -> Result<RoleDefinitions> {
        let mut stream = self
            .beta_client
            .role_management()
            .directory()
            .list_role_definitions()
            .paging()
            .stream::<RoleDefinitions>()
            .unwrap();

        let mut roles = RoleDefinitions::new();
        while let Some(result) = stream.next().await {
            let response = result.unwrap();

            let body = response.into_body().unwrap();

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
            .stream::<Groups>()
            .unwrap();

        let mut groups = Groups::default();
        while let Some(result) = stream.next().await {
            let response = result.unwrap();

            let body = response.into_body()?;

            groups.inner.extend(body.inner);
        }
        Ok(groups)
    }

    pub async fn list_users_channel(
        &self,
        splunk: &Splunk,
        sender: UnboundedSender<UsersMap<'_>>,
    ) -> Result<()> {
        let mut stream = self
            .beta_client
            .users()
            .list_user()
            .select(&[
                "id",
                "displayName",
                "givenName",
                "surname",
                "userPrincipalName",
                "transitiveMemberOf",
                "assignedPlans",
                "onPremisesSyncEnabled",
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

            splunk
                .log(&format!(
                    "Getting users batch {}, total users: {}",
                    batch, total_users
                ))
                .await
                .expect("Unable to log");
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

    pub async fn get_security_secure_scores(&self) -> Result<SecurityScores> {
        let response = self
            .beta_client
            .security()
            .list_secure_scores()
            .top("1")
            .send()
            .await?;
        let body: SecurityScores = response.json().await?;
        Ok(body)
    }

    pub async fn get_domains(&self) -> Result<Domains> {
        let response = self.client.domains().list_domain().send().await?;
        let mut body: Domains = response.json().await?;
        for domain in body.value.iter_mut() {
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

    pub async fn get_permission_grant_policy(&self) -> Result<PermissionGrantPolicy> {
        let response = self
            .client
            .policies()
            .list_permission_grant_policies()
            .send()
            .await?;
        let body = response.json().await?;
        Ok(body)
    }

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
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct AdminFormSettings {
    #[serde(rename = "settings")]
    inner: serde_json::Value,
}

impl ToHecEvents for &AdminFormSettings {
    type Item = Self;
    fn source(&self) -> &str {
        "msgraph"
    }

    fn sourcetype(&self) -> &str {
        "m365:admin_form_settings"
    }

    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        Box::new(iter::once(self))
    }
}

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

    fn to_hec_events(&self) -> anyhow::Result<Vec<crate::splunk::HecEvent>> {
        Ok(vec![HecEvent::new(
            &self,
            self.source(),
            self.sourcetype(),
        )?])
    }

    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        unimplemented!()
    }
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct MsGraphBatch<T> {
    responses: Vec<MsGraphResponse<T>>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct MsGraphResponse<T> {
    body: T,
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
}

// #[derive(Debug, Serialize, Deserialize, Default)]
// pub struct Domains(serde_json::Value);

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Domains {
    #[serde(rename = "@odata.context")]
    pub odata_context: String,
    pub value: Vec<Domain>,
}

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
    type Item = Self;
    fn source(&self) -> &str {
        "msgraph"
    }

    fn sourcetype(&self) -> &str {
        "m365:domains"
    }

    fn to_hec_events(&self) -> anyhow::Result<Vec<crate::splunk::HecEvent>> {
        Ok(vec![HecEvent::new(
            &self,
            self.source(),
            self.sourcetype(),
        )?])
    }
    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        unimplemented!()
    }
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct PermissionGrantPolicy(serde_json::Value);

impl ToHecEvents for &PermissionGrantPolicy {
    type Item = Self;
    fn source(&self) -> &'static str {
        "msgraph"
    }

    fn sourcetype(&self) -> &'static str {
        "m365:permission_grant_policy"
    }

    fn to_hec_events(&self) -> anyhow::Result<Vec<crate::splunk::HecEvent>> {
        Ok(vec![HecEvent::new(
            &self,
            self.source(),
            self.sourcetype(),
        )?])
    }

    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        unimplemented!()
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

    fn to_hec_events(&self) -> anyhow::Result<Vec<crate::splunk::HecEvent>> {
        Ok(vec![HecEvent::new(
            &self,
            self.source(),
            self.sourcetype(),
        )?])
    }

    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        unimplemented!()
    }

    // fn collection(&self) -> Box<dyn Iterator<Item = &&Self::Item>> {
    //     Box::new(vec![self].iter())
    // }
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

/// AAD + Azure
pub async fn azure_users(secrets: Arc<Secrets>, splunk: Arc<Splunk>) -> Result<()> {
    set_ssphp_run()?;

    splunk.log("Starting Azure Users collection").await?;
    splunk
        .log(&format!("GIT_HASH: {}", env!("GIT_HASH")))
        .await?;

    let ms_graph = login(
        &secrets.azure_client_id,
        &secrets.azure_client_secret,
        &secrets.azure_tenant_id,
    )
    .await?;

    let azure_rest = AzureRest::new(
        &secrets.azure_client_id,
        &secrets.azure_client_secret,
        &secrets.azure_tenant_id,
    )
    .await?;

    splunk.log("Azure logged in").await?;

    let (sender, mut reciever) = tokio::sync::mpsc::unbounded_channel::<UsersMap>();

    splunk.log("Getting Azure users").await?;

    let splunk_clone = splunk.clone();
    let ms_graph_clone = ms_graph.clone();
    let list_users = tokio::spawn(async move {
        ms_graph_clone
            .list_users_channel(&splunk_clone, sender)
            .await?;
        anyhow::Ok::<()>(())
    });

    let ms_graph_clone = ms_graph.clone();
    let splunk_clone = splunk.clone();

    splunk.log("Getting Azure Subscriptions").await?;
    let subscriptions = azure_rest.azure_subscriptions().await?;
    splunk.send_batch((&subscriptions).to_hec_events()?).await?;

    splunk
        .log("Getting Azure Subscription RoleDefinitions")
        .await?;
    let subscription_role_definitions = azure_rest.azure_role_definitions().await?;
    splunk
        .send_batch((&subscription_role_definitions).to_hec_events()?)
        .await?;

    splunk
        .log("Getting Azure Subscription RoleAssignments")
        .await?;
    let subscription_role_assignments = azure_rest.azure_role_assignments().await?;
    splunk
        .send_batch((&subscription_role_assignments).to_hec_events()?)
        .await?;

    splunk
        .log("Getting AAD Conditional access policies")
        .await?;
    let caps = ms_graph.list_conditional_access_policies().await?;
    splunk.send_batch((&caps).to_hec_events()?).await?;

    splunk.log("Getting AAD roles definitions").await?;
    let aad_role_definitions = ms_graph.list_role_definitions().await?;
    splunk
        .send_batch((&aad_role_definitions).to_hec_events()?)
        .await?;

    let process_to_splunk = tokio::spawn(async move {
        while let Some(mut users) = reciever.recv().await {
            users.set_is_privileged(&aad_role_definitions);

            users.process_caps(&caps);

            users
                .add_azure_roles(
                    &subscription_role_assignments,
                    &subscription_role_definitions,
                )
                .context("Failed to add azure roles")?;

            //            splunk.send_batch(&users.to_hec_events()?[..]).await?;
        }
        anyhow::Ok::<()>(())
    });

    let admin_request_consent_policy = ms_graph_clone
        .get_admin_request_consent_policy()
        .await
        .unwrap();

    splunk_clone
        .send_batch((&admin_request_consent_policy).to_hec_events().unwrap())
        .await?;

    let _ = list_users.await?;

    let _ = process_to_splunk.await?;

    Ok(())
}

pub async fn m365(secrets: Arc<Secrets>, splunk: Arc<Splunk>) -> Result<()> {
    //    let secrets = get_keyvault_secrets(&env::var("KEY_VAULT_NAME")?).await?;

    set_ssphp_run()?;

    //    let splunk = Splunk::new(&secrets.splunk_host, &secrets.splunk_token)?;
    splunk.log("Starting M365 collection").await?;
    splunk
        .log(&format!("GIT_HASH: {}", env!("GIT_HASH")))
        .await?;

    let ms_graph = login(
        &secrets.azure_client_id,
        &secrets.azure_client_secret,
        &secrets.azure_tenant_id,
    )
    .await?;

    let azure_rest = AzureRest::new(
        &secrets.azure_client_id,
        &secrets.azure_client_secret,
        &secrets.azure_tenant_id,
    )
    .await?;

    splunk.log("MS Graph logged in").await?;

    // TODO move this into another function
    splunk.log("Getting SecurityScores").await?;

    // match ms_graph.get_security_secure_scores().await {
    //     Ok(mut security_scores) => {
    //         let security_score = security_scores
    //             .value
    //             .first_mut()
    //             .context("Unable to get first SecurityScore")?;
    //         security_score.odata_context = Some(security_scores.odata_context.to_owned());
    //         let batch = security_score
    //             .control_scores
    //             .iter()
    //             .map(|cs| cs.to_hec_event().unwrap())
    //             .collect::<Vec<HecEvent>>();
    //         splunk.send_batch(&batch[..]).await?;
    //         splunk.send_batch(&[security_score.to_hec_event()?]).await?;
    //     }
    //     Err(error) => {
    //         splunk
    //             .log(&format!("Failed to get SecurityScores: {}", error))
    //             .await?;
    //     }
    // }

    // 5.1.1
    try_collect_send(
        "MS Graph Group Settings",
        ms_graph.get_access_reviews(),
        &splunk,
    )
    .await?;

    // 1.1.9
    // 1.1.10
    try_collect_send(
        "MS Graph Group Settings",
        ms_graph.get_admin_form_settings(),
        &splunk,
    )
    .await?;

    // 1.1.9
    // 1.1.10
    try_collect_send(
        "MS Graph Group Settings",
        ms_graph.list_group_settings(),
        &splunk,
    )
    .await?;

    try_collect_send(
        "MS Graph Security Secure Scores",
        ms_graph.get_security_secure_scores(),
        &splunk,
    )
    .await?;

    try_collect_send(
        "MS Graph Authentication Methods Policy",
        ms_graph.get_authentication_methods_policy(),
        &splunk,
    )
    .await?;

    try_collect_send(
        "MS Graph Authorization Policy",
        ms_graph.get_authorization_policy(),
        &splunk,
    )
    .await?;

    try_collect_send(
        "MS Graph Admin RequestConsent Policy",
        ms_graph.get_admin_request_consent_policy(),
        &splunk,
    )
    .await?;

    try_collect_send("MS Graph Domains", ms_graph.get_domains(), &splunk).await?;

    try_collect_send(
        "MS Graph Permission Grant Policy",
        ms_graph.get_permission_grant_policy(),
        &splunk,
    )
    .await?;

    try_collect_send("MS Graph Groups", ms_graph.list_groups(), &splunk).await?;

    try_collect_send(
        "Exchange Get Email Tenant Settings",
        run_powershell_get_email_tenant_settings(&secrets),
        &splunk,
    )
    .await?;

    try_collect_send(
        "Exchange Get Security Default Policy",
        ms_graph.get_identity_security_defaults_enforcement_policy(),
        &splunk,
    )
    .await?;

    try_collect_send(
        "Exchange Orgainization Config",
        run_powershell_get_organization_config(&secrets),
        &splunk,
    )
    .await?;

    try_collect_send(
        "Exchange Sharing Policy",
        run_powershell_get_sharing_policy(&secrets),
        &splunk,
    )
    .await?;

    try_collect_send(
        "Exchange Safe Links Policy",
        run_powershell_get_safe_links_policy(&secrets),
        &splunk,
    )
    .await?;

    try_collect_send(
        "Exchange Malware Filter Policy",
        run_powershell_get_malware_filter_policy(&secrets),
        &splunk,
    )
    .await?;

    try_collect_send(
        "Exchange Hosted Outbound Spam Filter Policy",
        run_powershell_get_hosted_outbound_spam_filter_policy(&secrets),
        &splunk,
    )
    .await?;

    try_collect_send(
        "Exchange Anti Phish Policy",
        run_powershell_get_anti_phish_policy(&secrets),
        &splunk,
    )
    .await?;

    try_collect_send(
        "Exchange Admin Audit Log Config",
        run_powershell_get_admin_audit_log_config(&secrets),
        &splunk,
    )
    .await?;

    try_collect_send(
        "Exchange OWA Mailbox Policy",
        run_powershell_get_owa_mailbox_policy(&secrets),
        &splunk,
    )
    .await?;

    try_collect_send(
        "Exchange Safe Attachment Policy",
        run_powershell_get_safe_attachment_policy(&secrets),
        &splunk,
    )
    .await?;

    try_collect_send(
        "Exchange ATP Polciy for O365",
        run_powershell_get_atp_policy_for_o365(&secrets),
        &splunk,
    )
    .await?;

    try_collect_send(
        "Exchange DLP Complaince Policy",
        run_powershell_get_dlp_compliance_policy(&secrets),
        &splunk,
    )
    .await?;

    try_collect_send(
        "Exchange Transport Rule",
        run_powershell_get_transport_rule(&secrets),
        &splunk,
    )
    .await?;

    try_collect_send(
        "Exchange Dkim Signing Config",
        run_powershell_get_dkim_signing_config(&secrets),
        &splunk,
    )
    .await?;

    try_collect_send(
        "Exchange Spoof Intelligence Insight",
        run_powershell_get_spoof_intelligence_insight(&secrets),
        &splunk,
    )
    .await?;

    try_collect_send(
        "Exchange Blocked Sender Address",
        run_powershell_get_blocked_sender_address(&secrets),
        &splunk,
    )
    .await?;

    try_collect_send(
        "Azure Security Contacts",
        azure_rest.get_security_contacts(),
        &splunk,
    )
    .await?;

    try_collect_send(
        "Azure Security Center built in",
        azure_rest.get_security_center_built_in(),
        &splunk,
    )
    .await?;

    try_collect_send(
        "Azure Security Auto Provisioning Settings",
        azure_rest.get_microsoft_security_auto_provisioning_settings(),
        &splunk,
    )
    .await?;

    try_collect_send(
        "Azure Security Settings",
        azure_rest.get_microsoft_security_settings(),
        &splunk,
    )
    .await?;

    try_collect_send(
        "Azure Security SQL Encryption protection",
        azure_rest.get_microsoft_sql_encryption_protection(),
        &splunk,
    )
    .await?;

    splunk.log("M365 Collection Complete").await?;

    Ok(())
}

async fn try_collect_send<T>(
    name: &str,
    future: impl Future<Output = Result<T>>,
    splunk: &Splunk,
) -> Result<()>
where
    for<'a> &'a T: ToHecEvents + Debug,
{
    splunk.log(&format!("Getting {}", &name)).await?;
    match future.await {
        Ok(ref result) => {
            let hec_events = match result.to_hec_events() {
                Ok(hec_events) => hec_events,
                Err(e) => {
                    eprintln!("Failed converting to HecEvents: {}", e);
                    dbg!(&result);
                    vec![HecEvent::new(
                        &Message {
                            event: format!("Failed converting to HecEvents: {}", e),
                        },
                        "data_ingester_rust",
                        "data_ingester_rust_logs",
                    )?]
                }
            };

            match splunk.send_batch(&hec_events).await {
                Ok(_) => eprintln!("Sent to Splunk"),
                Err(e) => {
                    eprintln!("Failed Sending to Splunk: {}", e);
                    //dbg!(&hec_events);
                }
            };
        }
        Err(err) => {
            splunk
                .log(&format!("Failed to get {}: {}", &name, err))
                .await?
        }
    };
    Ok(())
}

#[cfg(test)]
pub(crate) mod test {
    use std::env;

    use super::{login, MsGraph};
    use crate::{
        keyvault::get_keyvault_secrets,
        splunk::{set_ssphp_run, Splunk, ToHecEvents},
        users::UsersMap,
    };
    use anyhow::{Context, Result};

    pub async fn setup() -> Result<(Splunk, MsGraph)> {
        let secrets = get_keyvault_secrets(&env::var("KEY_VAULT_NAME")?).await?;

        set_ssphp_run()?;
        let ms_graph = login(
            &secrets.azure_client_id,
            &secrets.azure_client_secret,
            &secrets.azure_tenant_id,
        )
        .await?;
        let splunk = Splunk::new(&secrets.splunk_host, &secrets.splunk_token)?;
        Ok((splunk, ms_graph))
    }

    #[tokio::test]
    async fn get_users_channel() -> Result<()> {
        let (splunk, ms_graph) = setup().await?;

        let (sender, mut reciever) = tokio::sync::mpsc::unbounded_channel::<UsersMap>();

        let splunk_clone = splunk.clone();
        let ms_graph_clone = ms_graph.clone();
        let list_users = tokio::spawn(async move {
            ms_graph_clone
                .list_users_channel(&splunk_clone, sender)
                .await?;
            anyhow::Ok::<()>(())
        });

        let mut users_map = UsersMap::default();
        while let Some(users) = reciever.recv().await {
            users_map.extend(users);
        }
        let _ = list_users.await?;

        assert!(!users_map.inner.is_empty());
        // splunk
        //     .send_batch(&users_map.to_hec_events()?)
        //     .await?;
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
        let permission_grant_policy = ms_graph.get_permission_grant_policy().await?;
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
    async fn get_security_secure_scores() -> Result<()> {
        let (splunk, ms_graph) = setup().await?;
        let mut security_scores = ms_graph.get_security_secure_scores().await?;
        let security_score = security_scores
            .inner
            .first_mut()
            .context("Unable to get first SecrurityScore")?;
        security_score.odata_context = Some(security_scores.odata_context.to_owned());
        let batch = &*security_score;
        splunk.send_batch(batch.to_hec_events()?).await?;
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
}
