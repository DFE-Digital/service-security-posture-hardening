use crate::admin_request_consent_policy::AdminRequestConsentPolicy;
use crate::conditional_access_policies::ConditionalAccessPolicies;
use crate::directory_roles::DirectoryRoleTemplates;
use crate::directory_roles::DirectoryRoles;
use crate::groups::Groups;
use crate::keyvault::get_keyvault_secrets;
use crate::keyvault::Secrets;
use crate::powershell::run_powershell_get_admin_audit_log_config;
use crate::powershell::run_powershell_get_anti_phish_policy;
use crate::powershell::run_powershell_get_hosted_outbound_spam_filter_policy;
use crate::powershell::run_powershell_get_malware_filter_policy;
use crate::powershell::run_powershell_get_organization_config;
use crate::powershell::run_powershell_get_owa_mailbox_policy;
use crate::powershell::run_powershell_get_safe_links_policy;
use crate::roles::RoleDefinitions;
use crate::security_score::SecurityScores;
use crate::splunk::HecEvent;
use crate::splunk::ToHecEvent;
use crate::splunk::ToHecEvents;
use crate::splunk::{set_ssphp_run, Splunk};
use crate::users::Users;
use anyhow::{Context, Result};
use futures::Future;
use futures::StreamExt;
use graph_rs_sdk::oauth::AccessToken;
use graph_rs_sdk::oauth::OAuth;
use graph_rs_sdk::Graph;
use graph_rs_sdk::ODataQuery;
use serde::Deserialize;
use serde::Serialize;
use std::env;
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
        //println!("{access_token:#?}");
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
        //        oauth,
    })
}

#[derive(Clone)]
pub struct MsGraph {
    client: Graph,
    beta_client: Graph,
    //    oauth: OAuth,
}

impl MsGraph {
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
            // println!("{:#?}", response.json());
            // println!("{:#?}", response);

            let body = response.into_body();
            // println!("{:#?}", body);

            caps.value.extend(body.unwrap().value)
        }
        Ok(caps)
    }

    pub async fn list_directory_roles(&self) -> DirectoryRoles {
        let mut stream = self
            .client
            .directory_roles()
            .list_directory_role()
            .expand(&["members"])
            .paging()
            .stream::<DirectoryRoles>()
            .unwrap();

        let mut directory_roles = DirectoryRoles::new();
        while let Some(result) = stream.next().await {
            let response = result.unwrap();
            // println!("{:#?}", response);

            let body = response.into_body();
            // println!("{:#?}", body);

            directory_roles.value.extend(body.unwrap().value)
        }
        directory_roles
    }

    pub async fn list_directory_role_templates(&self) -> DirectoryRoleTemplates {
        let mut stream = self
            .client
            .directory_role_templates()
            .list_directory_role_template()
            .paging()
            .stream::<DirectoryRoleTemplates>()
            .unwrap();

        let mut directory_role_templates = DirectoryRoleTemplates::new();
        while let Some(result) = stream.next().await {
            let response = result.unwrap();
            // println!("{:#?}", response);

            let body = response.into_body();
            // println!("{:#?}", body);

            directory_role_templates.value.extend(body.unwrap().value)
        }
        directory_role_templates
    }

    pub async fn list_groups(&self) -> Groups {
        let mut stream = self
            .client
            .groups()
            .list_group()
            .top("1")
            .paging()
            .stream::<Groups>()
            .unwrap();

        let mut groups = Groups::new();
        while let Some(result) = stream.next().await {
            let response = result.unwrap();
            // println!("{:#?}", response);

            let body = response.into_body();
            // println!("{:#?}", body);

            groups.value.extend(body.unwrap().value)
        }
        groups
    }

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
            // println!("{:#?}", response);

            let body = response.into_body().unwrap();
            // println!("{:#?}", body);

            roles.value.extend(body.value)
        }
        Ok(roles)
    }

    pub async fn list_users(&self, splunk: &Splunk) -> Result<Users> {
        let mut stream = self
            .beta_client
            .users()
            .list_user()
            // .filter(&[&format!("startswith(userPrincipalName, '{}')", "")])
            // .order_by(&["userPrincipalName"])
            .select(&[
                "id",
                "displayName",
                "givenName",
                "surname",
                "userPrincipalName",
                "transitiveMemberOf",
                "assignedPlans",
            ])
            .expand(&["transitiveMemberOf"])
            .top("999")
            //            .skip("3")
            .paging()
            .stream::<Users>()?;

        let mut users = Users::new();
        let mut batch = 1;
        while let Some(result) = stream.next().await {
            let response = result?;
            // println!("{:#?}", response.json());
            // println!("{:#?}", response);

            let body = response.into_body();
            // println!("{:#?}", body);

            users.value.extend(body?.value);
            splunk
                .log(&format!(
                    "Getting users batch {}, total users: {}",
                    batch,
                    users.value.len()
                ))
                .await
                .expect("Unable to log");
            batch += 1;
        }
        Ok(users)
    }

    pub async fn list_users_channel(
        &self,
        splunk: &Splunk,
        sender: UnboundedSender<Users<'_>>,
    ) -> Result<()> {
        let mut stream = self
            .beta_client
            .users()
            .list_user()
            // .filter(&[&format!("startswith(userPrincipalName, '{}')", "")])
            // .order_by(&["userPrincipalName"])
            .select(&[
                "id",
                "displayName",
                "givenName",
                "surname",
                "userPrincipalName",
                "transitiveMemberOf",
                "assignedPlans",
            ])
            .expand(&["transitiveMemberOf"])
            .top("999")
            //            .skip("3")
            .paging()
            .stream::<Users>()?;

        let mut total_users = 0;
        let mut batch = 1;
        while let Some(result) = stream.next().await {
            let response = result?;
            //println!("{:#?}", response.json());

            let users = response.into_body()?;
            // println!("{:#?}", body);
            total_users += users.value.len();
            splunk
                .log(&format!(
                    "Getting users batch {}, total users: {}",
                    batch, total_users
                ))
                .await
                .expect("Unable to log");
            batch += 1;

            sender.send(users).expect("Unable to send Users to channel");
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
        let body = response.json().await?;
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
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct AuthorizationPolicy {
    #[serde(rename = "@odata.context")]
    pub odata_context: String,
    #[serde(flatten)]
    value: serde_json::Value,
}

impl ToHecEvent for AuthorizationPolicy {
    fn source() -> &'static str {
        "msgraph"
    }

    fn sourcetype() -> &'static str {
        "m365:authorization_policy"
    }
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Domains(serde_json::Value);

impl ToHecEvent for Domains {
    fn source() -> &'static str {
        "msgraph"
    }

    fn sourcetype() -> &'static str {
        "m365:domains"
    }
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct PermissionGrantPolicy(serde_json::Value);

impl ToHecEvent for PermissionGrantPolicy {
    fn source() -> &'static str {
        "msgraph"
    }

    fn sourcetype() -> &'static str {
        "m365:permission_grant_policy"
    }
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct IdentitySecurityDefaultsEnforcementPolicy(serde_json::Value);

impl ToHecEvent for IdentitySecurityDefaultsEnforcementPolicy {
    fn source() -> &'static str {
        "msgraph"
    }

    fn sourcetype() -> &'static str {
        "m365:identitySecurityDefaultsEnforcementPolicy"
    }
}

#[cfg(test)]
mod test {
    use std::env;

    use super::{login, MsGraph};
    use crate::{
        keyvault::get_keyvault_secrets,
        splunk::{set_ssphp_run, HecEvent, Splunk, ToHecEvent},
    };
    use anyhow::{Context, Result};

    async fn setup() -> Result<(Splunk, MsGraph)> {
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
    async fn get_admin_request_consent_policy() -> Result<()> {
        let (splunk, ms_graph) = setup().await?;
        let admin_request_consent_policy = ms_graph.get_admin_request_consent_policy().await?;
        splunk
            .send_batch(&[admin_request_consent_policy.to_hec_event()?])
            .await?;
        Ok(())
    }

    #[tokio::test]
    async fn get_authorization_policy() -> Result<()> {
        let (splunk, ms_graph) = setup().await?;
        let get_authorization_policy = ms_graph.get_authorization_policy().await?;
        splunk
            .send_batch(&[get_authorization_policy.to_hec_event()?])
            .await?;
        Ok(())
    }

    #[tokio::test]
    async fn get_domains() -> Result<()> {
        let (splunk, ms_graph) = setup().await?;
        let domains = ms_graph.get_domains().await?;
        splunk.send_batch(&[domains.to_hec_event()?]).await?;
        Ok(())
    }

    #[tokio::test]
    async fn get_permission_grant_policy() -> Result<()> {
        let (splunk, ms_graph) = setup().await?;
        let domains = ms_graph.get_permission_grant_policy().await?;
        splunk.send_batch(&[domains.to_hec_event()?]).await?;
        Ok(())
    }

    #[tokio::test]
    async fn get_identity_security_defaults_enforcement_policy() -> Result<()> {
        let (splunk, ms_graph) = setup().await?;
        let domains = ms_graph
            .get_identity_security_defaults_enforcement_policy()
            .await?;
        splunk.send_batch(&[domains.to_hec_event()?]).await?;
        Ok(())
    }

    #[tokio::test]
    async fn get_security_secure_scores() -> Result<()> {
        let (splunk, ms_graph) = setup().await?;
        let mut security_scores = ms_graph.get_security_secure_scores().await?;
        let mut security_score = security_scores
            .value
            .first_mut()
            .context("Unable to get first SecrurityScore")?;
        security_score.odata_context = Some(security_scores.odata_context.to_owned());
        let batch = security_score
            .control_scores
            .iter()
            .map(|cs| cs.to_hec_event().unwrap())
            .collect::<Vec<HecEvent>>();
        splunk.send_batch(&batch[..]).await?;
        splunk.send_batch(&[security_score.to_hec_event()?]).await?;
        Ok(())
    }
}

pub async fn azure(secrets: Arc<Secrets>, splunk: Arc<Splunk>) -> Result<()> {
    //    let secrets = get_keyvault_secrets(&env::var("KEY_VAULT_NAME")?).await?;

    set_ssphp_run()?;

    //  let splunk = Splunk::new(&secrets.splunk_host, &secrets.splunk_token)?;
    splunk.log("Starting Azure collection").await?;
    splunk
        .log(&format!("GIT_HASH: {}", env!("GIT_HASH")))
        .await?;

    let ms_graph = login(
        &secrets.azure_client_id,
        &secrets.azure_client_secret,
        &secrets.azure_tenant_id,
    )
    .await?;

    splunk.log("Azure logged in").await?;

    let (sender, mut reciever) = tokio::sync::mpsc::unbounded_channel::<Users>();

    splunk.log("Getting users").await?;

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
    let process_to_splunk = tokio::spawn(async move {
        splunk.log("Getting roles definitions").await?;
        let role_definitions = ms_graph.list_role_definitions().await?;

        splunk.log("Getting Conditional access policies").await?;
        let caps = ms_graph.list_conditional_access_policies().await?;

        while let Some(mut users) = reciever.recv().await {
            users.set_is_privileged(&role_definitions);
            users.process_caps(&caps);

            splunk.send_batch(&users.to_hec_eventss()?[..]).await?;
        }
        splunk.log("Users sent / Azure Complete").await?;
        anyhow::Ok::<()>(())
    });

    let admin_request_consent_policy = ms_graph_clone
        .get_admin_request_consent_policy()
        .await
        .unwrap();
    splunk_clone
        .send_batch(&[admin_request_consent_policy.to_hec_event().unwrap()])
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

    splunk.log("MS Graph logged in").await?;

    // TODO move this into another function
    splunk.log("Getting SecurityScores").await?;
    match ms_graph.get_security_secure_scores().await {
        Ok(mut security_scores) => {
            let security_score = security_scores
                .value
                .first_mut()
                .context("Unable to get first SecurityScore")?;
            security_score.odata_context = Some(security_scores.odata_context.to_owned());
            let batch = security_score
                .control_scores
                .iter()
                .map(|cs| cs.to_hec_event().unwrap())
                .collect::<Vec<HecEvent>>();
            splunk.send_batch(&batch[..]).await?;
            splunk.send_batch(&[security_score.to_hec_event()?]).await?;
        }
        Err(error) => {
            splunk
                .log(&format!("Failed to get SecurityScores: {}", error))
                .await?;
        }
    }

    try_collect(
        "MS Graph Authorization Policy",
        ms_graph.get_authorization_policy(),
        &splunk,
    )
    .await?;
    try_collect(
        "MS Graph Admin RequestConsent Policy",
        ms_graph.get_admin_request_consent_policy(),
        &splunk,
    )
    .await?;
    try_collect("MS Graph Domains", ms_graph.get_domains(), &splunk).await?;
    try_collect(
        "MS Graph Permission Grant Policy",
        ms_graph.get_permission_grant_policy(),
        &splunk,
    )
    .await?;
    try_collect(
        "Exchange Get Security Default Policy",
        ms_graph.get_identity_security_defaults_enforcement_policy(),
        &splunk,
    )
    .await?;
    try_collect(
        "Exchange Orgainization Config",
        run_powershell_get_organization_config(&secrets),
        &splunk,
    )
    .await?;
    try_collect(
        "Exchange Sharing Policy",
        run_powershell_get_organization_config(&secrets),
        &splunk,
    )
    .await?;
    try_collect(
        "Exchange Safe Links Policy",
        run_powershell_get_safe_links_policy(&secrets),
        &splunk,
    )
    .await?;
    try_collect(
        "Exchange Malware Filter Policy",
        run_powershell_get_malware_filter_policy(&secrets),
        &splunk,
    )
    .await?;
    try_collect(
        "Exchange Hosted Outbound Spam Filter Policy",
        run_powershell_get_hosted_outbound_spam_filter_policy(&secrets),
        &splunk,
    )
    .await?;
    try_collect(
        "Exchange Anti Phish Policy",
        run_powershell_get_anti_phish_policy(&secrets),
        &splunk,
    )
    .await?;
    try_collect(
        "Exchange Admin Audit Log Config",
        run_powershell_get_admin_audit_log_config(&secrets),
        &splunk,
    )
    .await?;
    try_collect(
        "Exchange OWA Mailbox Policy",
        run_powershell_get_owa_mailbox_policy(&secrets),
        &splunk,
    )
    .await?;

    splunk.log("M365 Collection Complete").await?;

    Ok(())
}

async fn try_collect<T: ToHecEvent>(
    name: &str,
    future: impl Future<Output = Result<T, anyhow::Error>>,
    splunk: &Splunk,
) -> Result<()> {
    splunk.log(&format!("Getting {}", &name)).await?;
    match future.await {
        Ok(result) => splunk.send_batch(&[result.to_hec_event()?]).await?,
        Err(err) => {
            splunk
                .log(&format!("Failed to get {}: {}", &name, err))
                .await?
        }
    };
    Ok(())
}
