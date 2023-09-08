use std::env;
use std::error::Error;

use crate::conditional_access_policies::ConditionalAccessPolicies;
use crate::directory_roles::DirectoryRoleTemplates;
use crate::directory_roles::DirectoryRoles;
use crate::groups::Groups;
use crate::keyvault::get_keyvault_secrets;
use crate::roles::RoleDefinitions;
use crate::splunk::Splunk;
use crate::users::Users;
use futures::StreamExt;
//use graph_rs_sdk::http::HttpResponseExt;
use graph_rs_sdk::oauth::AccessToken;
use graph_rs_sdk::oauth::OAuth;
use graph_rs_sdk::Graph;
use graph_rs_sdk::ODataQuery;

pub async fn login(
    client_id: &str,
    client_secret: &str,
    tenant_id: &str,
) -> Result<MsGraph, Box<dyn Error>> {
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
            .ok_or("no access token")?
            .bearer_token(),
    );
    let mut beta_client = Graph::new(
        oauth
            .get_access_token()
            .ok_or("no access token")?
            .bearer_token(),
    );
    beta_client.use_beta();

    Ok(MsGraph {
        client,
        beta_client,
        //        oauth,
    })
}

pub struct MsGraph {
    client: Graph,
    beta_client: Graph,
    //    oauth: OAuth,
}

impl MsGraph {
    pub async fn list_conditional_access_policies(&self) -> ConditionalAccessPolicies {
        let mut stream = self
            .client
            .identity()
            .list_policies()
            .paging()
            .stream::<ConditionalAccessPolicies>()
            .unwrap();

        let mut caps = ConditionalAccessPolicies::new();
        while let Some(result) = stream.next().await {
            let response = result.unwrap();
            // println!("{:#?}", response.json());
            // println!("{:#?}", response);

            let body = response.into_body();
            // println!("{:#?}", body);

            caps.value.extend(body.unwrap().value)
        }
        caps
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

    pub async fn list_role_definitions(&self) -> RoleDefinitions {
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

            let body = response.into_body();
            // println!("{:#?}", body);

            roles.value.extend(body.unwrap().value)
        }
        roles
    }

    pub async fn list_users(&self) -> Users {
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
            ])
            .expand(&["transitiveMemberOf"])
            .top("1")
            .paging()
            .stream::<Users>()
            .unwrap();

        let mut users = Users::new();
        while let Some(result) = stream.next().await {
            let response = result.unwrap();
            // println!("{:#?}", response.json());
            // println!("{:#?}", response);

            let body = response.into_body();
            // println!("{:#?}", body);

            users.value.extend(body.unwrap().value)
        }
        users
    }
}


pub async fn azure() -> Result<(), Box<dyn Error>> {
    let secrets = get_keyvault_secrets(
        &env::var("KEY_VAULT_NAME")?,
    )
    .await?;

    let splunk = Splunk::new(&secrets.splunk_host, &secrets.splunk_token)?;

    let ms_graph = login(
        &secrets.azure_client_id,
        &secrets.azure_client_secret,
        &secrets.azure_tenant_id,
    )
    .await?;

    let mut hec_events = vec![];

    // let directory_role_templates = ms_graph.list_directory_role_templates().await;
    // hec_events.extend(directory_role_templates.to_hec_event().into_iter());

    let role_definitions = ms_graph.list_role_definitions().await;
    // hec_events.extend(role_definitions.to_hec_event().into_iter());

    // let groups = ms_graph.list_groups().await;
    // hec_events.extend(groups.to_hec_event().into_iter());

    // let roles = ms_graph.list_directory_roles().await;
    // hec_events.extend(roles.to_hec_event().into_iter());

    let caps = ms_graph.list_conditional_access_policies().await;
    // hec_events.extend(caps.to_hec_event().into_iter());

    let mut users = ms_graph.list_users().await;
    users.process_caps(&caps);

    users.set_is_privileged(&role_definitions);
    hec_events.extend(users.to_hec_event().into_iter());

    splunk.send_batch(&hec_events).await;
    Ok(())
}
