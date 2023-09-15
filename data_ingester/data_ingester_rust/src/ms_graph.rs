use crate::admin_request_consent_policy::AdminRequestConsentPolicy;
use crate::conditional_access_policies::ConditionalAccessPolicies;
use crate::directory_roles::DirectoryRoleTemplates;
use crate::directory_roles::DirectoryRoles;
use crate::groups::Groups;
use crate::keyvault::get_keyvault_secrets;
use crate::roles::RoleDefinitions;
use crate::splunk::ToHecEvent;
use crate::splunk::ToHecEvents;
use crate::splunk::{set_ssphp_run, Splunk};
use crate::users::Users;
use anyhow::{bail, Context, Result};
use futures::StreamExt;
use graph_rs_sdk::oauth::AccessToken;
use graph_rs_sdk::oauth::OAuth;
use graph_rs_sdk::Graph;
use graph_rs_sdk::ODataQuery;
use std::env;
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

    pub async fn get_admin_request_consent_policy(&self) -> Result<AdminRequestConsentPolicy> {
        let mut stream = self
            .client
            .policies()
            .get_admin_consent_request_policy()
            .paging()
            .stream::<AdminRequestConsentPolicy>()
            .unwrap();

        //        let mut roles = RoleDefinitions::new();
        while let Some(result) = stream.next().await {
            let response = result.unwrap();
            println!("{:#?}", response);

            let body = response.into_body().unwrap();
            println!("{:#?}", body);

            return Ok(body);
        }
        // TODO
        bail!("not ok")
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
            // println!("{:#?}", response.json());

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
}

#[tokio::test]
async fn get_admin_request_consent_policy() {
    let secrets = get_keyvault_secrets(&env::var("KEY_VAULT_NAME").unwrap())
        .await
        .unwrap();

    set_ssphp_run().unwrap();
    let ms_graph = login(
        &secrets.azure_client_id,
        &secrets.azure_client_secret,
        &secrets.azure_tenant_id,
    )
    .await
    .unwrap();
    let admin_request_consent_policy = ms_graph.get_admin_request_consent_policy().await.unwrap();

    let splunk = Splunk::new(&secrets.splunk_host, &secrets.splunk_token).unwrap();
    splunk
        .send_batch(&[admin_request_consent_policy.to_hec_event().unwrap()])
        .await
        .unwrap();
}

pub async fn azure() -> Result<()> {
    let secrets = get_keyvault_secrets(&env::var("KEY_VAULT_NAME")?).await?;

    set_ssphp_run()?;

    let splunk = Splunk::new(&secrets.splunk_host, &secrets.splunk_token)?;
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

    //    let mut hec_events = vec![];

    // let directory_role_templates = ms_graph.list_directory_role_templates().await;
    // hec_events.extend(directory_role_templates.to_hec_event().into_iter());

    // splunk.log("Getting roles definitions").await;
    // let role_definitions = ms_graph.list_role_definitions().await?;
    // dbg!(&role_definitions);
    // hec_events.extend(role_definitions.to_hec_event().into_iter());

    // let groups = ms_graph.list_groups().await;
    // hec_events.extend(groups.to_hec_event().into_iter());

    // let roles = ms_graph.list_directory_roles().await;
    // hec_events.extend(roles.to_hec_event().into_iter());

    // splunk.log("Getting Conditional access policies").await;
    // let caps = ms_graph.list_conditional_access_policies().await;
    // dbg!(&caps);
    // hec_events.extend(caps.to_hec_event().into_iter());

    // splunk.log("Getting users").await;
    // let mut users = ms_graph.list_users(&splunk).await?;
    // //    dbg!(&users);

    // splunk.log("Processing CAPs").await;
    // users.process_caps(&caps);

    // splunk.log("Processing user is privileged").await;
    // users.set_is_privileged(&role_definitions);
    // hec_events.extend(users.to_hec_eventss()?.into_iter());
    // //    dbg!(&hec_events);
    // splunk.log("sending users").await;
    // splunk.send_batch(&hec_events[..]).await;
    // splunk.log("Users sent / Azure Complete").await;

    let (sender, mut reciever) = tokio::sync::mpsc::unbounded_channel::<Users>();

    splunk.log("Getting users").await?;
    // let mut users = ms_graph.list_users_channel(&splunk, sender).await?;
    // dbg!(&users);
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

pub async fn m365() -> Result<()> {
    let secrets = get_keyvault_secrets(&env::var("KEY_VAULT_NAME")?).await?;

    set_ssphp_run()?;

    let splunk = Splunk::new(&secrets.splunk_host, &secrets.splunk_token)?;
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

    splunk.log("Getting users").await?;

    let admin_request_consent_policy = ms_graph.get_admin_request_consent_policy().await.unwrap();
    splunk
        .send_batch(&[admin_request_consent_policy.to_hec_event().unwrap()])
        .await?;

    splunk.log("M365 Collection Complete").await?;

    Ok(())
}
