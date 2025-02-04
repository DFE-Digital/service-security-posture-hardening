use crate::ado_response::{AdoMetadata, AdoRateLimiting, AdoResponse};
use crate::data::organization::Organizations;
use anyhow::{Context, Result};
use serde::Deserialize;
use tracing::{error, trace};

pub(crate) struct AzureDevOpsClient {
    pub(crate) client: reqwest::Client,
    token: Token,
    api_version: String,
    pub(crate) tenant_id: String,
}

#[derive(Debug, Deserialize)]
struct Token {
    #[allow(unused)]
    token_type: TokenType,
    #[allow(unused)]
    expires_in: usize,
    #[allow(unused)]
    ext_expires_in: usize,
    access_token: String,
}

#[derive(Debug, Deserialize)]
enum TokenType {
    Bearer,
}

impl AzureDevOpsClient {
    pub(crate) async fn new(client_id: &str, client_secret: &str, tenant_id: &str) -> Result<Self> {
        let client = reqwest::Client::new();
        let url = format!("https://login.microsoftonline.com/{tenant_id}/oauth2/v2.0/token");
        let params = [
            ("client_id", client_id),
            ("client_secret", client_secret),
            ("grant_type", "client_credentials"),
            // Fixed ADO scope
            ("scope", "499b84ac-1321-427f-aa17-267ca6975798/.default"),
        ];
        let response = client.post(url).form(&params).send().await?;

        let token = response
            .json()
            .await
            .context("Getting JSON from Oauth request")?;

        Ok(Self {
            client,
            api_version: "7.2-preview.1".into(),
            token,
            tenant_id: tenant_id.into(),
        })
    }

    #[allow(unused)]
    async fn post(
        &self,
        url: &str,
        body: String,
        organization: &str,
        r#type: &str,
        rest_docs: &str,
    ) -> Result<AdoResponse> {
        let response = self
            .client
            .post(url)
            .body(body)
            .bearer_auth(&self.token.access_token)
            .send()
            .await?;

        if !response.status().is_success() {
            error!(name="Azure Dev Ops", operation="POST request", error="Non 2xx status code", status=?response.status(), headers=?response.headers());
            anyhow::bail!("failed request");
        }

        let rate_limit = AdoRateLimiting::from_headers(response.headers());
        trace!(rate_limit=?rate_limit);

        let ado_metadata = AdoMetadata::new(
            &self.tenant_id,
            url,
            Some(organization),
            response.status().as_u16(),
            r#type,
            rest_docs,
        );

        let ado_response = {
            let mut ado_response = response.json::<AdoResponse>().await?;
            ado_response.metadata = Some(ado_metadata);
            ado_response
        };

        Ok(ado_response)
    }

    async fn get(
        &self,
        url: &str,
        organization: &str,
        r#type: &str,
        rest_docs: &str,
    ) -> Result<AdoResponse> {
        let response = self
            .client
            .get(url)
            .bearer_auth(&self.token.access_token)
            .send()
            .await?;

        if !response.status().is_success() {
            error!(name="Azure Dev Ops", operation="GET request", error="Non 2xx status code", status=?response.status(), headers=?response.headers());
            anyhow::bail!("failed request");
        }

        let rate_limit = AdoRateLimiting::from_headers(response.headers());
        error!(rate_limit=?rate_limit);

        let ado_metadata = AdoMetadata::new(
            &self.tenant_id,
            url,
            Some(organization),
            response.status().as_u16(),
            r#type,
            rest_docs,
        );

        let text = response.text().await?;
        trace!(
            name = "Azure Dev Ops",
            operation = "get response",
            response = text
        );

        let ado_response = {
            let mut ado_response: AdoResponse = serde_json::from_str(&text)?;
            // response.json::<AdoResponse>().await?;
            ado_response.metadata = Some(ado_metadata);
            ado_response
        };

        Ok(ado_response)
    }

    pub(crate) async fn organizations_list(&self) -> Result<Organizations> {
        let url = format!(
            "https://aexprodcus1.vsaex.visualstudio.com/_apis/EnterpriseCatalog/Organizations?tenantId={}",
            self.tenant_id
        );

        let response = self
            .client
            .get(&url)
            .bearer_auth(&self.token.access_token)
            .send()
            .await?;

        if !response.status().is_success() {
            error!(name="Azure Dev Ops", operation="organizations_list GET request", error="Non 2xx status code", status=?response.status(), headers=?response.headers());
            anyhow::bail!("failed request");
        }

        let rate_limit = AdoRateLimiting::from_headers(response.headers());
        trace!(rate_limit=?rate_limit);

        let ado_metadata = AdoMetadata::new(
            &self.tenant_id,
            &url,
            None,
            response.status().as_u16(),
            "fn organizations_list",
            "no REST docs",
        );

        let text = response.text().await?;

        let organizations = Organizations::from_csv(&text, ado_metadata);

        Ok(organizations)
    }

    pub(crate) async fn projects_list(&self, organization: &str) -> Result<AdoResponse> {
        let url = format!(
            "https://dev.azure.com/{organization}/_apis/projects?api-version={}",
            self.api_version
        );
        self.get(&url, organization, "fn projects_list", "https://learn.microsoft.com/en-us/rest/api/azure/devops/core/projects/list?view=azure-devops-rest-7.1&tabs=HTTP").await
    }

    pub(crate) async fn git_policy_configuration_get(
        &self,
        organization: &str,
        project: &str,
    ) -> Result<AdoResponse> {
        let url = format!(
            "https://dev.azure.com/{organization}/{project}/_apis/git/policy/configurations?api-version={}",
            self.api_version
        );
        self.get(&url, organization, "fn git_policy_configuration_get", "https://learn.microsoft.com/en-us/rest/api/azure/devops/git/repositories/list?view=azure-devops-rest-7.2&tabs=HTTP").await
    }

    pub(crate) async fn git_repository_list(
        &self,
        organization: &str,
        project: &str,
    ) -> Result<AdoResponse> {
        let url = format!(
            "https://dev.azure.com/{organization}/{project}/_apis/git/repositories?api-version={}",
            self.api_version
        );
        self.get(&url, organization, "fn git_repository_list", "https://learn.microsoft.com/en-us/rest/api/azure/devops/git/repositories/list?view=azure-devops-rest-7.1&tabs=HTTP").await
    }

    // pub(crate) async fn accounts_list(
    //     &self,
    //     organization: &str,
    //     owner_id: &str,
    // ) -> Result<AdoResponse> {
    //     let url = format!(
    //         "https://dev.azure.com/{organization}/_apis/accounts?ownerId={owner_id}&api-version=5.0-preview.1"
    //     );
    //     self.get(&url, &organization, "accounts", "").await
    // }

    // async fn admin_overview(&self, organization: &str) -> Result<Vec<Organization>> {
    //     let body = json!({
    //     "contributionIds": ["ms.vss-admin-web.organization-admin-overview-delay-load-data-provider"],
    //     "dataProviderContext": {
    //         "properties": {
    //             "sourcePage": {
    //                 "routeId": "ms.vss-admin-web.collection-admin-hub-route",
    //                 "routeValues": {
    //                     "action": "Execute",
    //                     "adminPivot": "organizationOverview",
    //                     "controller": "ContributedPage",
    //                     "serviceHost": "71645052-a9cf-4f92-8075-3b018969bf4d (aktest0831)"
    //                 },
    //                 "url": "https://dev.azure.com/aktest0831/_settings/organizationOverview"
    //             }}}});

    //     let other_body = json!({
    //     "contributionIds":["ms.vss-admin-web.organization-admin-overview-delay-load-data-provider"],
    //     "dataProviderContext":{
    //         "properties":{
    //             "sourcePage":{
    //                 "url":"https://dev.azure.com/aktest0831/_settings/organizationOverview",
    //                 "routeId":"ms.vss-admin-web.collection-admin-hub-route",
    //                 "routeValues":{
    //                     "adminPivot":"organizationOverview",
    //                     "controller":"ContributedPage",
    //                     "action":"Execute",
    //                     "serviceHost":"71645052-a9cf-4f92-8075-3b018969bf4d (aktest0831)"
    //                 }}}}});
    //     dbg!(&body);
    //     dbg!(&other_body);
    //     assert_eq!(body, other_body);

    //     let url = format!(
    //         "https://dev.azure.com/{organization}/_apis/Contribution/HierarchyQuery?api-version={}",
    //         self.api_version
    //     );
    //     let response = self
    //         .client
    //         .post(url)
    //         .header("Content-Type", "application/json")
    //         .json(&body)
    //         .bearer_auth(&self.token.access_token)
    //         .send()
    //         .await?;
    //     //let result = self.post(&url, body.to_string(), organization).await?;
    //     dbg!(&response);
    //     // println!("{}", &response.text().await?);
    //     let text = response.text().await?;
    //     dbg!(&text);
    //     let value = serde_json::from_str::<Value>(&text)?;
    //     dbg!("{#?}", &value);
    //     let mut response_body = serde_json::from_str::<Root>(&text)?;
    //     dbg!(&response_body);
    //     Ok(response_body
    //         .data_providers
    //         .ms_vss_features_my_organizations_data_provider
    //         .take()
    //         .unwrap()
    //         .organizations)
    // }

    // /// FFS MS - https://stackoverflow.com/questions/54762368/get-all-organizations-in-azure-devops-using-rest-api
    // async fn org_details(&self, organization: &str) -> Result<Vec<Organization>> {
    //     let body = r#"{
    //         "contributionIds": ["ms.vss-features.my-organizations-data-provider"],
    //         "dataProviderContext": {
    //             "properties": {}
    //         }}"#;
    //     let url = format!(
    //         "https://dev.azure.com/{organization}/_apis/Contribution/HierarchyQuery?api-version={}",
    //         self.api_version
    //     );
    //     let response = self
    //         .client
    //         .post(url)
    //         .header("Content-Type", "application/json")
    //         .body(body)
    //         .bearer_auth(&self.token.access_token)
    //         .send()
    //         .await?;
    //     //let result = self.post(&url, body.to_string(), organization).await?;
    //     dbg!(&response);
    //     // println!("{}", &response.text().await?);
    //     let mut response_body = response.json::<Root>().await?;
    //     Ok(response_body
    //         .data_providers
    //         .ms_vss_features_my_organizations_data_provider
    //         .take()
    //         .unwrap()
    //         .organizations)
    // }
}

// #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
// #[serde(rename_all = "camelCase")]
// pub struct Root {
//     pub data_provider_shared_data: DataProviderSharedData,
//     pub data_providers: DataProviders,
// }

// #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
// #[serde(rename_all = "camelCase")]
// pub struct DataProviderSharedData {
//     #[serde(rename = "_featureFlags")]
//     pub feature_flags: FeatureFlags,
// }

// #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
// #[serde(rename_all = "camelCase")]
// pub struct FeatureFlags {
//     #[serde(
//         rename = "VisualStudio.Services.AdminEngagement.OrganizationOverview.EditableOrganizationAvatar"
//     )]
//     pub visual_studio_services_admin_engagement_organization_overview_editable_organization_avatar:
//         bool,
// }

// #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
// #[serde(rename_all = "camelCase")]
// pub struct DataProviders {
//     #[serde(rename = "ms.vss-web.component-data")]
//     pub ms_vss_web_component_data: MsVssWebComponentData,
//     #[serde(rename = "ms.vss-web.shared-data")]
//     pub ms_vss_web_shared_data: Value,
//     #[serde(rename = "ms.vss-features.my-organizations-data-provider")]
//     pub ms_vss_features_my_organizations_data_provider:
//         Option<MsVssFeaturesMyOrganizationsDataProvider>,
//     #[serde(rename = "ms.vss-admin-web.organization-admin-overview-delay-load-data-provider")]
//     pub ms_vss_admin_web_organization_admin_overview_delay_load_data_provider:
//         Option<MsVssAdminWebOrganizationAdminOverviewDelayLoadDataProvider>,
// }

// #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
// #[serde(rename_all = "camelCase")]
// struct MsVssAdminWebOrganizationAdminOverviewDelayLoadDataProvider {
//     pub is_owner: bool,
//     pub has_delete_permissions: bool,
//     pub has_modify_permissions: bool,
//     pub current_owner: CurrentOwner,
//     pub current_user_id: String,
//     pub organization_takeover: OrganizationTakeover,
//     pub all_time_zones: Vec<AllTimeZone>,
//     pub show_domain_migration: bool,
//     pub disable_domain_migration: bool,
//     pub dev_ops_domain_urls: bool,
//     pub target_domain_url: String,
//     pub avatar_url: String,
// }

// #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
// #[serde(rename_all = "camelCase")]
// pub struct CurrentOwner {
//     pub name: String,
//     pub id: String,
//     pub email: String,
// }

// #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
// #[serde(rename_all = "camelCase")]
// pub struct OrganizationTakeover {}

// #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
// #[serde(rename_all = "camelCase")]
// pub struct AllTimeZone {
//     pub display_name: String,
//     pub id: String,
// }

// #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
// #[serde(rename_all = "camelCase")]
// pub struct MsVssWebComponentData {}

// #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
// #[serde(rename_all = "camelCase")]
// pub struct MsVssFeaturesMyOrganizationsDataProvider {
//     pub organizations: Vec<Organization>,
//     pub most_recently_accessed_hosts: Vec<Value>,
//     pub create_new_org_url: String,
//     pub is_user_account_mapping_required: bool,
// }

// #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
// #[serde(rename_all = "camelCase")]
// pub struct Organization {
//     pub id: String,
//     pub name: String,
//     pub url: String,
//     pub avatar_url: String,
// }

// struct HeirarchyQuery {
//     dataProviderSharedData:
// }

#[cfg(test)]
#[cfg(feature = "live_tests")]
mod live_tests {
    use crate::test_utils::TEST_SETUP;
    use anyhow::Result;
    use data_ingester_splunk::splunk::{Splunk, ToHecEvents};

    async fn send_to_splunk(splunks: &[Splunk], ado_response: impl ToHecEvents) -> Result<()> {
        let hec_events = (ado_response).to_hec_events()?;

        for splunk in splunks {
            splunk.send_batch(hec_events.clone()).await?;
        }

        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        Ok(())
    }

    #[test]
    fn test_all() {
        let t = &*TEST_SETUP;
        let _: Result<()> = t.runtime.block_on(async {
            let projects = t.ado.projects_list(&t.organization).await?;
            send_to_splunk(&t.splunks, projects).await?;

            let policy_configuration = t
                .ado
                .git_policy_configuration_get(&t.organization, "foo")
                .await?;
            send_to_splunk(&t.splunks, policy_configuration).await?;

            let result = t.ado.git_repository_list(&t.organization, "foo").await?;
            send_to_splunk(&t.splunks, result).await?;
            Ok(())
        });
    }

    #[test]
    fn test_ado_projects_list() {
        let t = &*TEST_SETUP;
        let _: Result<()> = t.runtime.block_on(async {
            let projects = t.ado.projects_list(&t.organization).await?;
            assert!(!projects.value.is_empty());
            assert_eq!(projects.count, projects.value.len());
            send_to_splunk(&t.splunks, projects).await?;
            Ok(())
        });
    }

    #[test]
    fn test_ado_git_policy_configuration_get() {
        let t = &*TEST_SETUP;
        let _: Result<()> = t.runtime.block_on(async {
            let policy_configuration = t
                .ado
                .git_policy_configuration_get(&t.organization, "foo")
                .await?;

            assert!(!policy_configuration.value.is_empty());
            assert_eq!(policy_configuration.count, policy_configuration.value.len());
            send_to_splunk(&t.splunks, policy_configuration).await?;
            Ok(())
        });
    }

    #[test]
    fn test_ado_git_repository_list() {
        let t = &*TEST_SETUP;
        let _: Result<()> = t.runtime.block_on(async {
            let result = t.ado.git_repository_list(&t.organization, "foo").await?;

            assert!(!result.value.is_empty());
            assert_eq!(result.count, result.value.len());
            send_to_splunk(&t.splunks, result).await?;
            Ok(())
        });
    }

    #[test]
    fn test_ado_organizations_list() {
        let t = &*TEST_SETUP;
        let _: Result<()> = t.runtime.block_on(async {
            let result = t.ado.organizations_list().await?;
            assert!(!result.organizations.is_empty());
            send_to_splunk(&t.splunks, &result).await?;
            Ok(())
        });
    }

    // #[tokio::test]
    // async fn test_ado_accounts_list() -> Result<()> {
    //     let t = test_setup().await;
    //     let owner_id = t
    //         .ado
    //         .org_details(&t.organization)
    //         .await?
    //         .first()
    //         .unwrap()
    //         .id
    //         .to_string();
    //     let accounts = t.ado.accounts_list(&t.organization, &owner_id).await?;
    //     assert!(accounts.value.len() > 0);
    //     send_to_splunk(&t.splunks, accounts).await?;
    //     Ok(())
    // }

    // #[tokio::test]
    // async fn test_ado_owner_id() -> Result<()> {
    //     let t = test_setup().await;
    //     let owner_id = t.ado.org_details(&t.organization).await?;
    //     // send_to_splunk(&t.splunks, accounts).await?;
    //     Ok(())
    // }

    // #[tokio::test]
    // async fn test_admin_overview() -> Result<()> {
    //     let t = test_setup().await;
    //     let owner_id = t.ado.admin_overview(&t.organization).await?;
    //     assert!(false);
    //     // send_to_splunk(&splunks, accounts).await?;
    //     Ok(())
    // }
}
