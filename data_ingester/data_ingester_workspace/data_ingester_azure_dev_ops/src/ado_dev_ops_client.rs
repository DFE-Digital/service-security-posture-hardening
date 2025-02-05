use crate::ado_metadata::{AdoMetadata, AdoMetadataBuilder};
use crate::ado_response::{
    AddAdoResponse, AdoPaging, AdoRateLimiting, AdoResponse, AdoResponseSingle,
};
use crate::data::organization::Organizations;
use anyhow::{Context, Result};
use serde::de::DeserializeOwned;
use serde::Deserialize;
use tracing::{debug, error, trace};

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

    // async fn post(
    //     &self,
    //     url: &str,
    //     body: String,
    //     organization: &str,
    //     project: Option<&str>,
    //     repo: Option<&str>,
    //     r#type: &str,
    //     rest_docs: &str,
    // ) -> Result<AdoResponse> {
    //     let response = self
    //         .client
    //         .post(url)
    //         .body(body)
    //         .bearer_auth(&self.token.access_token)
    //         .send()
    //         .await?;

    //     if !response.status().is_success() {
    //         error!(name="Azure Dev Ops", operation="POST request", error="Non 2xx status code", status=?response.status(), headers=?response.headers());
    //         anyhow::bail!("failed request");
    //     }

    //     let rate_limit = AdoRateLimiting::from_headers(response.headers());
    //     trace!(rate_limit=?rate_limit);

    //     let ado_metadata = {
    //         let mut metadata_builder = AdoMetadataBuilder::new()
    //             .tenant(&self.tenant_id)
    //             .url(url)
    //             .organization(organization)
    //             .r#type(r#type)
    //             .rest_docs(rest_docs);

    //         metadata_builder = if let Some(project) = project {
    //             metadata_builder.project(project)
    //         } else {
    //             metadata_builder
    //         };

    //         metadata_builder = if let Some(repo) = repo {
    //             metadata_builder.repo(repo)
    //         } else {
    //             metadata_builder
    //         };

    //         metadata_builder.build()
    //     };

    //     let ado_response = {
    //         let mut ado_response = response.json::<AdoResponse>().await?;
    //         ado_response.metadata = Some(ado_metadata);
    //         ado_response
    //     };

    //     Ok(ado_response)
    // }

    async fn get<T: DeserializeOwned + AddAdoResponse>(
        &self,
        mut metadata: AdoMetadata,
    ) -> Result<AdoResponse> {
        let mut continuation_token = AdoPaging::default();
        let mut collection = AdoResponse::default();

        loop {
            let next_url = if continuation_token.has_more() {
                format!(
                    "{}&continuationToken={}",
                    metadata.url(),
                    continuation_token.next_token()
                )
            } else {
                metadata.url().to_string()
            };

            let response = self
                .client
                .get(&next_url)
                .bearer_auth(&self.token.access_token)
                .send()
                .await?;

            let status = response.status();
            let headers = response.headers().clone();
            let text = response.text().await?;

            if !status.is_success() {
                error!(name="Azure Dev Ops", operation="GET request", error="Non 2xx status code", status=?status, headers=?headers, body=text);
                anyhow::bail!("Azure Dev Org request failed with with Non 2xx status code");
            }
            metadata.status.push(status.into());

            let rate_limit = AdoRateLimiting::from_headers(&headers);
            debug!(rate_limit=?rate_limit);

            continuation_token = AdoPaging::from_headers(&headers);

            trace!(
                name = "Azure Dev Ops",
                operation = "get response",
                url = next_url,
                response = text
            );

            let ado_response: T = serde_json::from_str(&text)?;

            collection.count += ado_response.count();
            collection.value.extend(ado_response.values());

            if continuation_token.is_empty() {
                break;
            }
        }
        collection.metadata = Some(metadata);

        Ok(collection)
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

        let ado_metadata = AdoMetadataBuilder::new()
            .tenant(&self.tenant_id)
            .url(&url)
            .r#type("fn organizations_list")
            .rest_docs("no REST docs")
            .build();

        let text = response.text().await?;

        let organizations = Organizations::from_csv(&text, ado_metadata);

        Ok(organizations)
    }

    pub(crate) async fn projects_list(&self, organization: &str) -> Result<AdoResponse> {
        let url = format!(
            "https://dev.azure.com/{organization}/_apis/projects?api-version={}",
            self.api_version
        );

        let ado_metadata = AdoMetadataBuilder::new()
            .tenant(&self.tenant_id)
            .url(url)
            .organization(organization)
            .r#type("fn projects_list")
            .rest_docs("https://learn.microsoft.com/en-us/rest/api/azure/devops/core/projects/list?view=azure-devops-rest-7.1&tabs=HTTP")
            .build();

        self.get::<AdoResponse>(ado_metadata).await
    }

    pub(crate) async fn audit_streams(&self, organization: &str) -> Result<AdoResponse> {
        let url = format!(
            "https://auditservice.dev.azure.com/{organization}/_apis/audit/streams?api-version={}",
            self.api_version
        );

        let ado_metadata = AdoMetadataBuilder::new()
            .tenant(&self.tenant_id)
            .url(url)
            .organization(organization)
            .r#type("fn audit_streams")
            .rest_docs("https://learn.microsoft.com/en-us/rest/api/azure/devops/audit/streams/query-all-streams?view=azure-devops-rest-7.1&tabs=HTTP")
            .build();

        self.get::<AdoResponse>(ado_metadata).await
    }

    /// ADO is trash
    /// Response when trying to list PAT tokens for an org
    // {
    //   "$id": "1",
    //   "errorCode": 0,
    //   "eventId": 3000,
    //   "innerException": null,
    //   "message": "Service principals are not allowed to perform this action.",
    //   "typeKey": "InvalidAccessException",
    //   "typeName": "Microsoft.TeamFoundation.Framework.Server.InvalidAccessException, Microsoft.TeamFoundation.Framework.Server"
    // }
    #[allow(unused)]
    pub(crate) async fn pat_tokens(&self, organization: &str) -> Result<AdoResponse> {
        let url = format!(
            "https://vssps.dev.azure.com/{organization}/_apis/tokens/pats?api-version={}",
            self.api_version
        );
        let ado_metadata = AdoMetadataBuilder::new()
            .tenant(&self.tenant_id)
            .url(url)
            .organization(organization)
            .r#type("fn pat_tokens")
            .rest_docs("https://learn.microsoft.com/en-us/rest/api/azure/devops/tokens/pats/list?view=azure-devops-rest-7.1&tabs=HTTP")
            .build();

        self.get::<AdoResponse>(ado_metadata).await
    }

    pub(crate) async fn policy_configuration_get(
        &self,
        organization: &str,
        project: &str,
    ) -> Result<AdoResponse> {
        let url = format!(
            "https://dev.azure.com/{organization}/{project}/_apis/policy/configurations?api-version={}",
            self.api_version
        );
        let ado_metadata = AdoMetadataBuilder::new()
            .tenant(&self.tenant_id)
            .url(url)
            .organization(organization)
            .project(project)
            .r#type("fn policy_configuration_get")
            .rest_docs("https://learn.microsoft.com/en-us/rest/api/azure/devops/policy/configurations/list?view=azure-devops-rest-7.1&tabs=HTTP")
            .build();

        self.get::<AdoResponse>(ado_metadata).await
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
        let ado_metadata = AdoMetadataBuilder::new()
            .tenant(&self.tenant_id)
            .url(url)
            .organization(organization)
            .project(project)
            .r#type("fn git_policy_configuration_get")
            .rest_docs("https://learn.microsoft.com/en-us/rest/api/azure/devops/git/policy-configurations/get?view=azure-devops-rest-7.1")
            .build();

        self.get::<AdoResponse>(ado_metadata).await
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
        let ado_metadata = AdoMetadataBuilder::new()
            .tenant(&self.tenant_id)
            .url(url)
            .organization(organization)
            .project(project)
            .r#type("fn git_repository_list")
            .rest_docs("https://learn.microsoft.com/en-us/rest/api/azure/devops/git/repositories/list?view=azure-devops-rest-7.1&tabs=HTTP")
            .build();

        self.get::<AdoResponse>(ado_metadata).await
    }

    pub(crate) async fn graph_users_list(&self, organization: &str) -> Result<AdoResponse> {
        let url = format!(
            "https://vssps.dev.azure.com/{organization}/_apis/graph/users?api-version={}",
            self.api_version
        );

        let ado_metadata = AdoMetadataBuilder::new()
            .tenant(&self.tenant_id)
            .url(url)
            .organization(organization)
            .r#type("fn graph_users_list")
            .rest_docs("https://learn.microsoft.com/en-us/rest/api/azure/devops/graph/users/list?view=azure-devops-rest-7.1&tabs=HTTP")
            .build();

        self.get::<AdoResponse>(ado_metadata).await
    }

    pub(crate) async fn graph_service_principals_list(
        &self,
        organization: &str,
    ) -> Result<AdoResponse> {
        let url = format!(
            "https://vssps.dev.azure.com/{organization}/_apis/graph/serviceprincipals?api-version={}",
            self.api_version
        );
        let ado_metadata = AdoMetadataBuilder::new()
            .tenant(&self.tenant_id)
            .url(url)
            .organization(organization)
            .r#type("fn graph_service_principals_list")
            .rest_docs("https://learn.microsoft.com/en-us/rest/api/azure/devops/graph/service-principals/list?view=azure-devops-rest-7.1&tabs=HTTP")
            .build();

        self.get::<AdoResponse>(ado_metadata).await
    }

    pub(crate) async fn graph_groups_list(&self, organization: &str) -> Result<AdoResponse> {
        let url = format!(
            "https://vssps.dev.azure.com/{organization}/_apis/graph/groups?api-version={}",
            self.api_version
        );
        let ado_metadata = AdoMetadataBuilder::new()
            .tenant(&self.tenant_id)
            .url(url)
            .organization(organization)
            .r#type("fn graph_groups_list")
            .rest_docs("https://learn.microsoft.com/en-us/rest/api/azure/devops/graph/groups/list?view=azure-devops-rest-7.1&tabs=HTTP")
            .build();

        self.get::<AdoResponse>(ado_metadata).await
    }

    pub(crate) async fn adv_security_org_enablement(
        &self,
        organization: &str,
    ) -> Result<AdoResponse> {
        let url = format!(
            "https://advsec.dev.azure.com/{organization}/_apis/management/enablement?api-version={}&includeAllProperties=true",
            self.api_version
        );
        let ado_metadata = AdoMetadataBuilder::new()
            .tenant(&self.tenant_id)
            .url(url)
            .organization(organization)
            .r#type("fn adv_security_org_enablement")
            .rest_docs("https://learn.microsoft.com/en-us/rest/api/azure/devops/advancedsecurity/org-enablement/get?view=azure-devops-rest-7.2")
            .build();

        self.get::<AdoResponseSingle>(ado_metadata).await
    }

    pub(crate) async fn adv_security_project_enablement(
        &self,
        organization: &str,
        project: &str,
    ) -> Result<AdoResponse> {
        let url = format!(
            "https://advsec.dev.azure.com/{organization}/{project}/_apis/management/enablement?api-version={}&includeAllProperties=true",
            self.api_version
        );
        let ado_metadata = AdoMetadataBuilder::new()
            .tenant(&self.tenant_id)
            .url(url)
            .organization(organization)
            .project(project)
            .r#type("fn adv_security_project_enablement")
            .rest_docs("https://learn.microsoft.com/en-us/rest/api/azure/devops/advancedsecurity/project-enablement/get?view=azure-devops-rest-7.2")
            .build();

        self.get::<AdoResponseSingle>(ado_metadata).await
    }

    pub(crate) async fn adv_security_repo_enablement(
        &self,
        organization: &str,
        project: &str,
        repository: &str,
    ) -> Result<AdoResponse> {
        let url = format!(
            "https://advsec.dev.azure.com/{organization}/{project}/_apis/management/repositories/{repository}/enablement?api-version={}&includeAllProperties=true",
            self.api_version
        );

        let ado_metadata = AdoMetadataBuilder::new()
            .tenant(&self.tenant_id)
            .url(url)
            .organization(organization)
            .project(project)
            .repo(repository)
            .r#type("fn adv_security_repo_enablement")
            .rest_docs("https://learn.microsoft.com/en-us/rest/api/azure/devops/advancedsecurity/repo-enablement/get?view=azure-devops-rest-7.2")
            .build();

        self.get::<AdoResponseSingle>(ado_metadata).await
    }

    pub(crate) async fn adv_security_alerts(
        &self,
        organization: &str,
        project: &str,
        repository: &str,
    ) -> Result<AdoResponse> {
        let url = format!(
            "https://advsec.dev.azure.com/{organization}/{project}/_apis/alert/repositories/{repository}/alerts?api-version={}",
            self.api_version
        );

        let ado_metadata = AdoMetadataBuilder::new()
            .tenant(&self.tenant_id)
            .url(url)
            .organization(organization)
            .project(project)
            .repo(repository)
            .r#type("fn adv_security_alerts")
            .rest_docs("https://learn.microsoft.com/en-us/rest/api/azure/devops/advancedsecurity/alerts/list?view=azure-devops-rest-7.2")
            .build();

        self.get::<AdoResponse>(ado_metadata).await
    }

    pub(crate) async fn build_general_settings(
        &self,
        organization: &str,
        project: &str,
    ) -> Result<AdoResponse> {
        let url = format!(
            "https://dev.azure.com/{organization}/{project}/_apis/build/generalsettings?api-version={}",
            self.api_version
        );
        let ado_metadata = AdoMetadataBuilder::new()
            .tenant(&self.tenant_id)
            .url(url)
            .organization(organization)
            .project(project)
            .r#type("fn build_general_settings")
            .rest_docs("https://learn.microsoft.com/en-us/rest/api/azure/devops/build/general-settings/get?view=azure-devops-rest-7.2")
            .build();

        self.get::<AdoResponseSingle>(ado_metadata).await
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
                .git_policy_configuration_get(&t.organization, &t.project)
                .await?;
            send_to_splunk(&t.splunks, policy_configuration).await?;

            let result = t
                .ado
                .git_repository_list(&t.organization, &t.project)
                .await?;
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
    fn test_ado_audit_streams() {
        let t = &*TEST_SETUP;
        let result: Result<()> = t.runtime.block_on(async {
            let audit_streams = t.ado.audit_streams(&t.organization).await?;
            dbg!(&audit_streams);
            // assert!(false);
            assert!(!audit_streams.value.is_empty());
            assert_eq!(audit_streams.count, audit_streams.value.len());
            send_to_splunk(&t.splunks, audit_streams).await?;
            Ok(())
        });
        result.unwrap();
    }

    #[ignore = "PAT token auth is broken"]
    #[test]
    fn test_ado_pat_tokens() {
        let t = &*TEST_SETUP;
        let result: Result<()> = t.runtime.block_on(async {
            let pat_tokens = t.ado.pat_tokens(&t.organization).await?;
            assert!(!pat_tokens.value.is_empty());
            assert_eq!(pat_tokens.count, pat_tokens.value.len());
            send_to_splunk(&t.splunks, pat_tokens).await?;
            Ok(())
        });
        result.unwrap();
    }

    #[test]
    fn test_ado_policy_configuration_get() {
        let t = &*TEST_SETUP;
        let _: Result<()> = t.runtime.block_on(async {
            let policy_configuration = t
                .ado
                .policy_configuration_get(&t.organization, &t.project)
                .await?;

            assert!(!policy_configuration.value.is_empty());
            assert_eq!(policy_configuration.count, policy_configuration.value.len());
            send_to_splunk(&t.splunks, policy_configuration).await?;
            Ok(())
        });
    }

    #[test]
    fn test_ado_git_policy_configuration_get() {
        let t = &*TEST_SETUP;
        let _: Result<()> = t.runtime.block_on(async {
            let policy_configuration = t
                .ado
                .git_policy_configuration_get(&t.organization, &t.project)
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
            let result = t
                .ado
                .git_repository_list(&t.organization, &t.project)
                .await?;
            assert!(!result.value.is_empty());
            assert_eq!(result.count, result.value.len());
            send_to_splunk(&t.splunks, result).await?;
            Ok(())
        });
    }

    #[test]
    fn test_ado_organizations_list() {
        let t = &*TEST_SETUP;
        let result: Result<()> = t.runtime.block_on(async {
            let result = t.ado.organizations_list().await?;
            assert!(!result.organizations.is_empty());
            send_to_splunk(&t.splunks, &result).await?;
            Ok(())
        });
        result.unwrap();
    }

    #[test]
    fn test_ado_graph_users_list() {
        let t = &*TEST_SETUP;
        let result: Result<()> = t.runtime.block_on(async {
            let result = t.ado.graph_users_list(&t.organization).await?;
            assert!(!result.value.is_empty());
            send_to_splunk(&t.splunks, &result).await?;
            Ok(())
        });
        result.unwrap();
    }

    #[test]
    fn test_ado_graph_service_principals_list() {
        let t = &*TEST_SETUP;
        let result: Result<()> = t.runtime.block_on(async {
            let result = t.ado.graph_service_principals_list(&t.organization).await?;
            assert!(!result.value.is_empty());
            send_to_splunk(&t.splunks, &result).await?;
            Ok(())
        });
        result.unwrap();
    }

    #[test]
    fn test_ado_graph_groups_list() {
        let t = &*TEST_SETUP;
        let result: Result<()> = t.runtime.block_on(async {
            let result = t.ado.graph_groups_list(&t.organization).await?;
            assert!(!result.value.is_empty());
            send_to_splunk(&t.splunks, &result).await?;
            Ok(())
        });
        result.unwrap();
    }

    #[test]
    fn test_ado_adv_security_org_enablement() {
        let t = &*TEST_SETUP;
        let result: Result<()> = t.runtime.block_on(async {
            let result = t.ado.adv_security_org_enablement(&t.organization).await?;
            send_to_splunk(&t.splunks, &result).await?;
            assert!(!result.value.is_empty());
            Ok(())
        });
        result.unwrap();
    }

    #[test]
    fn test_ado_adv_security_project_enablement() {
        let t = &*TEST_SETUP;
        let result: Result<()> = t.runtime.block_on(async {
            let result = t
                .ado
                .adv_security_project_enablement(&t.organization, &t.project)
                .await?;
            send_to_splunk(&t.splunks, &result).await?;
            assert!(!result.value.is_empty());
            Ok(())
        });
        result.unwrap();
    }

    #[test]
    fn test_ado_adv_security_repo_enablement() {
        let t = &*TEST_SETUP;
        let result: Result<()> = t.runtime.block_on(async {
            let result = t
                .ado
                .adv_security_repo_enablement(&t.organization, &t.project, &t.repo)
                .await?;
            send_to_splunk(&t.splunks, &result).await?;
            assert!(!result.value.is_empty());
            Ok(())
        });
        result.unwrap();
    }

    #[ignore = "No Adv Security Alerts... For now"]
    #[test]
    fn test_ado_adv_security_alerts() {
        let t = &*TEST_SETUP;
        let result: Result<()> = t.runtime.block_on(async {
            let result = t
                .ado
                .adv_security_alerts(&t.organization, &t.project, &t.repo)
                .await?;
            send_to_splunk(&t.splunks, &result).await?;
            assert!(!result.value.is_empty());
            Ok(())
        });
        result.unwrap();
    }

    #[test]
    fn test_ado_build_general_settings() {
        let t = &*TEST_SETUP;
        let result: Result<()> = t.runtime.block_on(async {
            let result = t
                .ado
                .build_general_settings(&t.organization, &t.project)
                .await?;
            send_to_splunk(&t.splunks, &result).await?;
            assert!(!result.value.is_empty());
            Ok(())
        });
        result.unwrap();
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
