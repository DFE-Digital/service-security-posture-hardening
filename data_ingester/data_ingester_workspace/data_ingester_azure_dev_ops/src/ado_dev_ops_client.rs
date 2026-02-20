use crate::ado_metadata::{AdoMetadata, AdoMetadataBuilder, NoRestDocs, NoType, NoUrl};
use crate::ado_response::{AddAdoResponse, AdoResponse, AdoResponseSingle};
use crate::data::projects::Project;
use crate::data::repositories::Repository;
use anyhow::Result;
use serde::de::DeserializeOwned;

pub(crate) trait AzureDevOpsClient {
    async fn get<T: DeserializeOwned + serde::Serialize + AddAdoResponse>(
        &self,
        metadata: AdoMetadata,
    ) -> Result<AdoResponse>;

    fn ado_metadata_builder(&self) -> AdoMetadataBuilder<NoUrl, NoType, NoRestDocs> {
        AdoMetadataBuilder::new()
    }
}

pub(crate) trait AzureDevOpsClientMethods: AzureDevOpsClient {
    async fn projects_list(&self, organization: &str) -> Result<AdoResponse> {
        let api_version = "7.2-preview.4";
        let url = format!(
            "https://dev.azure.com/{organization}/_apis/projects?api-version={}",
            api_version
        );

        let ado_metadata = self.ado_metadata_builder()
            .url(url)
            .organization(organization)
            .r#type("fn projects_list")
            .rest_docs("https://learn.microsoft.com/en-us/rest/api/azure/devops/core/projects/list?view=azure-devops-rest-7.2")
            .build();

        self.get::<AdoResponse>(ado_metadata).await
    }

    async fn audit_streams(&self, organization: &str) -> Result<AdoResponse> {
        let api_version = "7.2-preview.1";
        let url = format!(
            "https://auditservice.dev.azure.com/{organization}/_apis/audit/streams?api-version={}",
            api_version
        );

        let ado_metadata = self.ado_metadata_builder()
            .url(url)
            .organization(organization)
            .r#type("fn audit_streams")
            .rest_docs("https://learn.microsoft.com/en-us/rest/api/azure/devops/audit/streams/query-all-streams?view=azure-devops-rest-7.2")
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
    async fn pat_tokens(&self, organization: &str) -> Result<AdoResponse> {
        let api_version = "7.2-preview.1";
        let url = format!(
            "https://vssps.dev.azure.com/{organization}/_apis/tokens/pats?api-version={}",
            api_version
        );
        let ado_metadata = self.ado_metadata_builder()
            .url(url)
            .organization(organization)
            .r#type("fn pat_tokens")
            .rest_docs("https://learn.microsoft.com/en-us/rest/api/azure/devops/tokens/pats/list?view=azure-devops-rest-7.2")
            .build();

        self.get::<AdoResponse>(ado_metadata).await
    }

    async fn policy_configuration_get(
        &self,
        organization: &str,
        project: &Project,
    ) -> Result<AdoResponse> {
        let api_version = "7.2-preview.1";
        let url = format!(
            "https://dev.azure.com/{organization}/{project_id}/_apis/policy/configurations?api-version={api_version}",
            project_id=project.id,
            api_version=api_version,
        );
        let ado_metadata = self.ado_metadata_builder()
            .url(url)
            .organization(organization)
            .project(project)
            .r#type("fn policy_configuration_get")
            .rest_docs("https://learn.microsoft.com/en-us/rest/api/azure/devops/policy/configurations/list?view=azure-devops-rest-7.2")
            .build();

        self.get::<AdoResponse>(ado_metadata).await
    }

    async fn policy_types_get(&self, organization: &str, project: &Project) -> Result<AdoResponse> {
        let api_version = "7.2-preview.1";
        let url = format!(
            "https://dev.azure.com/{organization}/{project_id}/_apis/policy/types?api-version={api_version}",
            project_id=project.id,
            api_version=api_version,
        );
        let ado_metadata = self.ado_metadata_builder()
            .url(url)
            .organization(organization)
            .project(project)
            .r#type("fn policy_types_get")
            .rest_docs("https://learn.microsoft.com/en-us/rest/api/azure/devops/policy/types/list?view=azure-devops-rest-7.2")
            .build();

        self.get::<AdoResponse>(ado_metadata).await
    }

    /// Retrieve a list of policy configurations by a given set of scope/filtering criteria.
    /// repositoryId unset, refName unset: returns all policy configurations that are defined at the project level
    async fn git_policy_configuration_get(
        &self,
        organization: &str,
        project: &Project,
    ) -> Result<AdoResponse> {
        let api_version = "7.2-preview.1";
        let url = format!(
            "https://dev.azure.com/{organization}/{project_id}/_apis/git/policy/configurations?api-version={api_version}",
            project_id=project.id,
            api_version=api_version,
        );
        let ado_metadata = self.ado_metadata_builder()
            .url(url)
            .organization(organization)
            .project(project)
            .r#type("fn git_policy_configuration_get")
            .rest_docs("https://learn.microsoft.com/en-us/rest/api/azure/devops/git/policy-configurations/get?view=azure-devops-rest-7.2")
            .build();

        self.get::<AdoResponse>(ado_metadata).await
    }

    /// Retrieve a list of policy configurations by a given set of scope/filtering criteria.
    /// repositoryId set, refName unset: returns all policy configurations that apply to a particular repository
    #[allow(unused)]
    async fn git_repo_policy_configuration_get(
        &self,
        organization: &str,
        project: &Project,
        repo: &Repository,
    ) -> Result<AdoResponse> {
        let api_version = "7.2-preview.1";
        let url = format!(
            "https://dev.azure.com/{organization}/{project_id}/_apis/git/policy/configurations?api-version={api_version}&repositoryId={repo_id}",
            project_id=project.id,
            repo_id=repo.id(),
            api_version=api_version,
        );
        let ado_metadata = self.ado_metadata_builder()
            .url(url)
            .organization(organization)
            .project(project)
            .repo(repo)
            .r#type("fn git_repo_policy_configuration_get")
            .rest_docs("https://learn.microsoft.com/en-us/rest/api/azure/devops/git/policy-configurations/get?view=azure-devops-rest-7.2")
            .build();

        self.get::<AdoResponse>(ado_metadata).await
    }

    async fn git_repository_list(
        &self,
        organization: &str,
        project: &Project,
    ) -> Result<AdoResponse> {
        let api_version = "7.2-preview.2";
        let url = format!(
            "https://dev.azure.com/{organization}/{project_id}/_apis/git/repositories?api-version={api_version}",
            project_id=project.id,
            api_version=api_version,
        );
        let ado_metadata = self.ado_metadata_builder()
            .url(url)
            .organization(organization)
            .project(project)
            .r#type("fn git_repository_list")
            .rest_docs("https://learn.microsoft.com/en-us/rest/api/azure/devops/git/repositories/list?view=azure-devops-rest-7.2")
            .build();

        self.get::<AdoResponse>(ado_metadata).await
    }

    async fn graph_users_list(&self, organization: &str) -> Result<AdoResponse> {
        let api_version = "7.2-preview.1";
        let url = format!(
            "https://vssps.dev.azure.com/{organization}/_apis/graph/users?api-version={}",
            api_version
        );

        let ado_metadata = self.ado_metadata_builder()
            .url(url)
            .organization(organization)
            .r#type("fn graph_users_list")
            .rest_docs("https://learn.microsoft.com/en-us/rest/api/azure/devops/graph/users/list?view=azure-devops-rest-7.2")
            .build();

        self.get::<AdoResponse>(ado_metadata).await
    }

    async fn graph_service_principals_list(&self, organization: &str) -> Result<AdoResponse> {
        let api_version = "7.2-preview.1";
        let url = format!(
            "https://vssps.dev.azure.com/{organization}/_apis/graph/serviceprincipals?api-version={api_version}",
            api_version=api_version
        );
        let ado_metadata = self.ado_metadata_builder()
            .url(url)
            .organization(organization)
            .r#type("fn graph_service_principals_list")
            .rest_docs("https://learn.microsoft.com/en-us/rest/api/azure/devops/graph/service-principals/list?view=azure-devops-rest-7.2")
            .build();

        self.get::<AdoResponse>(ado_metadata).await
    }

    async fn graph_groups_list(&self, organization: &str) -> Result<AdoResponse> {
        let api_version = "7.2-preview.1";
        let url = format!(
            "https://vssps.dev.azure.com/{organization}/_apis/graph/groups?api-version={api_version}",
            api_version=api_version
        );
        let ado_metadata = self.ado_metadata_builder()
            .url(url)
            .organization(organization)
            .r#type("fn graph_groups_list")
            .rest_docs("https://learn.microsoft.com/en-us/rest/api/azure/devops/graph/groups/list?view=azure-devops-rest-7.2")
            .build();

        self.get::<AdoResponse>(ado_metadata).await
    }

    async fn adv_security_org_enablement(&self, organization: &str) -> Result<AdoResponse> {
        let api_version = "7.2-preview.3";
        let url = format!(
            "https://advsec.dev.azure.com/{organization}/_apis/management/enablement?api-version={api_version}&includeAllProperties=true",
            api_version=api_version,
        );
        let ado_metadata = self.ado_metadata_builder()
            .url(url)
            .organization(organization)
            .r#type("fn adv_security_org_enablement")
            .rest_docs("https://learn.microsoft.com/en-us/rest/api/azure/devops/advancedsecurity/org-enablement/get?view=azure-devops-rest-7.2")
            .build();

        self.get::<AdoResponseSingle>(ado_metadata).await
    }

    async fn security_namespaces(&self, organization: &str) -> Result<AdoResponse> {
        let api_version = "7.2-preview.1";
        let url = format!(
            "https://dev.azure.com/{organization}/_apis/securitynamespaces?api-version={api_version}",
            api_version=api_version,
        );
        let ado_metadata = self.ado_metadata_builder()
            .url(url)
            .organization(organization)
            .r#type("fn security_namespaces")
            .rest_docs("https://learn.microsoft.com/en-us/rest/api/azure/devops/security/security-namespaces/query?view=azure-devops-rest-7.2")
            .build();

        self.get::<AdoResponse>(ado_metadata).await
    }

    async fn security_access_control_lists(
        &self,
        organization: &str,
        namespace_id: &str,
    ) -> Result<AdoResponse> {
        let api_version = "7.2-preview.1";
        let url = format!(
            "https://dev.azure.com/{organization}/_apis/accesscontrollists/{namespace_id}?api-version={api_version}&recurse=True&includeExtendedInfo=True",
            api_version=api_version
        );
        let ado_metadata = self.ado_metadata_builder()
            .url(url)
            .organization(organization)
            .r#type("fn security_access_control_lists")
            .rest_docs("https://learn.microsoft.com/en-us/rest/api/azure/devops/security/access-control-lists/query?view=azure-devops-rest-7.2")
            .build();

        self.get::<AdoResponse>(ado_metadata).await
    }

    async fn identities(&self, organization: &str, descriptor: &str) -> Result<AdoResponse> {
        let api_version = "7.2-preview.1";
        let url = format!(
            "https://vssps.dev.azure.com/{organization}/_apis/identities?api-version={api_version}&descriptors={descriptor}&queryMembership=Expanded",
            api_version=api_version
        );
        let ado_metadata = self.ado_metadata_builder()
            .url(url)
            .organization(organization)
            .r#type("fn identities")
            .rest_docs("https://learn.microsoft.com/en-us/rest/api/azure/devops/ims/identities/read-identities?view=azure-devops-rest-7.2")
            .build();

        self.get::<AdoResponse>(ado_metadata).await
    }

    async fn adv_security_project_enablement(
        &self,
        organization: &str,
        project: &Project,
    ) -> Result<AdoResponse> {
        let api_version = "7.2-preview.3";
        let url = format!(
            "https://advsec.dev.azure.com/{organization}/{project_id}/_apis/management/enablement?api-version={api_version}&includeAllProperties=true",
            project_id=project.id,
            api_version=api_version
        );
        let ado_metadata = self.ado_metadata_builder()
            .url(url)
            .organization(organization)
            .project(project)
            .r#type("fn adv_security_project_enablement")
            .rest_docs("https://learn.microsoft.com/en-us/rest/api/azure/devops/advancedsecurity/project-enablement/get?view=azure-devops-rest-7.2")
            .build();

        self.get::<AdoResponseSingle>(ado_metadata).await
    }

    async fn adv_security_repo_enablement(
        &self,
        organization: &str,
        project: &Project,
        repository: &Repository,
    ) -> Result<AdoResponse> {
        let api_version = "7.2-preview.3";
        let url = format!(
            "https://advsec.dev.azure.com/{organization}/{project_id}/_apis/management/repositories/{repository_id}/enablement?api-version={api_version}&includeAllProperties=true",
            project_id=project.id,
            repository_id=repository.id(),
            api_version=api_version,
        );

        let ado_metadata = self.ado_metadata_builder()
            .url(url)
            .organization(organization)
            .project(project)
            .repo(repository)
            .r#type("fn adv_security_repo_enablement")
            .rest_docs("https://learn.microsoft.com/en-us/rest/api/azure/devops/advancedsecurity/repo-enablement/get?view=azure-devops-rest-7.2")
            .build();

        self.get::<AdoResponseSingle>(ado_metadata).await
    }

    async fn adv_security_alerts(
        &self,
        organization: &str,
        project: &Project,
        repository: &Repository,
    ) -> Result<AdoResponse> {
        let api_version = "7.2-preview.1";
        let url = format!(
            "https://advsec.dev.azure.com/{organization}/{project_id}/_apis/alert/repositories/{repository_id}/alerts?api-version={api_version}",
            project_id=project.id,
            repository_id=repository.id(),
            api_version=api_version
        );

        let ado_metadata = self.ado_metadata_builder()
            .url(url)
            .organization(organization)
            .project(project)
            .repo(repository)
            .r#type("fn adv_security_alerts")
            .rest_docs("https://learn.microsoft.com/en-us/rest/api/azure/devops/advancedsecurity/alerts/list?view=azure-devops-rest-7.2")
            .build();

        self.get::<AdoResponse>(ado_metadata).await
    }

    async fn repo_stats_list(
        &self,
        organization: &str,
        project: &Project,
        repository: &Repository,
    ) -> Result<AdoResponse> {
        let api_version = "7.2-preview.2";
        let url = format!(
            "https://dev.azure.com/{organization}/{project_id}/_apis/git/repositories/{repository_id}/stats/branches?api-version={api_version}",
            project_id=project.id,
            repository_id=repository.id(),
            api_version=api_version,
        );

        let ado_metadata = self.ado_metadata_builder()
            .url(url)
            .organization(organization)
            .project(project)
            .repo(repository)
            .r#type("fn repo_stats_list")
            .rest_docs("https://learn.microsoft.com/en-us/rest/api/azure/devops/git/stats/list?view=azure-devops-rest-4.1&tabs=HTTP")
            .build();

        self.get::<AdoResponse>(ado_metadata).await
    }

    async fn build_general_settings(
        &self,
        organization: &str,
        project: &Project,
    ) -> Result<AdoResponse> {
        let api_version = "7.2-preview.1";
        let url = format!(
            "https://dev.azure.com/{organization}/{project_id}/_apis/build/generalsettings?api-version={api_version}",
            project_id=project.id,
            api_version=api_version,
        );
        let ado_metadata = self.ado_metadata_builder()
            .url(url)
            .organization(organization)
            .project(project)
            .r#type("fn build_general_settings")
            .rest_docs("https://learn.microsoft.com/en-us/rest/api/azure/devops/build/general-settings/get?view=azure-devops-rest-7.2")
            .build();

        self.get::<AdoResponseSingle>(ado_metadata).await
    }
}

#[cfg(test)]
mod tests {
    use crate::data::security_acl::test::acls_from_ado_response;
    use crate::test_utils::TEST_SETUP;
    use crate::{ado_dev_ops_client::AzureDevOpsClientMethods, ado_response::AdoResponse};
    use anyhow::Result;
    use data_ingester_splunk::splunk::{SplunkTrait, ToHecEvents};

    async fn send_to_splunk(
        splunk: &(impl SplunkTrait + Sync),
        ado_response: impl ToHecEvents,
    ) -> Result<()> {
        let hec_events = (ado_response).to_hec_events()?;

        splunk.send_batch(hec_events.clone()).await?;

        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        Ok(())
    }

    // #[test]
    // fn test_all() {
    //     let t = &*TEST_SETUP;
    //     let _: Result<()> = t.runtime.block_on(async {
    //         let projects = t.ado.projects_list(&t.organization).await?;
    //         send_to_splunk(&t.splunk, projects).await?;

    //         let policy_configuration = t
    //             .ado
    //             .git_policy_configuration_get(&t.organization, &t.project)
    //             .await?;
    //         send_to_splunk(&t.splunk, policy_configuration).await?;

    //         let result = t
    //             .ado
    //             .git_repository_list(&t.organization, &t.project)
    //             .await?;
    //         send_to_splunk(&t.splunk, result).await?;
    //         Ok(())
    //     });
    // }

    #[test]
    fn test_ado_projects_list() {
        let t = &*TEST_SETUP;
        let _: Result<()> = t.runtime.block_on(async {
            let projects = t.ado.projects_list(&t.organization).await?;
            assert!(!projects.value.is_empty());
            assert_eq!(projects.count, projects.value.len());
            send_to_splunk(&t.splunk, projects).await?;
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
            send_to_splunk(&t.splunk, audit_streams).await?;
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
            send_to_splunk(&t.splunk, pat_tokens).await?;
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
            send_to_splunk(&t.splunk, policy_configuration).await?;
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
            send_to_splunk(&t.splunk, policy_configuration).await?;
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
            send_to_splunk(&t.splunk, result).await?;
            Ok(())
        });
    }

    // #[test]
    // fn test_ado_organizations_list() {
    //     let t = &*TEST_SETUP;
    //     let result: Result<()> = t.runtime.block_on(async {
    //         let result = t.ado.organizations_list().await?;
    //         assert!(!result.organizations.is_empty());
    //         send_to_splunk(&t.splunk, &result).await?;
    //         Ok(())
    //     });
    //     result.unwrap();
    // }

    #[test]
    fn test_ado_graph_users_list() {
        let t = &*TEST_SETUP;
        let result: Result<()> = t.runtime.block_on(async {
            let result = t.ado.graph_users_list(&t.organization).await?;
            assert!(!result.value.is_empty());
            send_to_splunk(&t.splunk, &result).await?;
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
            send_to_splunk(&t.splunk, &result).await?;
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
            send_to_splunk(&t.splunk, &result).await?;
            Ok(())
        });
        result.unwrap();
    }

    #[test]
    fn test_ado_adv_security_org_enablement() {
        let t = &*TEST_SETUP;
        let result: Result<()> = t.runtime.block_on(async {
            let result = t.ado.adv_security_org_enablement(&t.organization).await?;
            send_to_splunk(&t.splunk, &result).await?;
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
            send_to_splunk(&t.splunk, &result).await?;
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
            send_to_splunk(&t.splunk, &result).await?;
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
            send_to_splunk(&t.splunk, &result).await?;
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
            send_to_splunk(&t.splunk, &result).await?;
            assert!(!result.value.is_empty());
            Ok(())
        });
        result.unwrap();
    }

    #[test]
    fn test_ado_security_namespaces() {
        let t = &*TEST_SETUP;
        let result: Result<()> = t.runtime.block_on(async {
            let result = t.ado.security_namespaces(&t.organization).await?;
            send_to_splunk(&t.splunk, &result).await?;
            assert!(!result.value.is_empty());
            Ok(())
        });
        result.unwrap();
    }

    #[test]
    pub fn test_ado_access_control_lists() {
        let t = &*TEST_SETUP;
        let namespaces = crate::data::security_namespaces::test::security_namespace();
        let namespace = namespaces
            .namespaces
            .iter()
            .find(|namespace| namespace.name == "Git Repositories")
            .unwrap();
        let result: Result<AdoResponse> = t.runtime.block_on(async {
            let result = t
                .ado
                .security_access_control_lists(&t.organization, &namespace.namespace_id)
                .await?;
            assert!(!result.value.is_empty());
            Ok(result)
        });
        let _ = result.unwrap();
    }

    #[test]
    pub fn test_ado_identities() {
        let t = &*TEST_SETUP;
        let acls = acls_from_ado_response();
        let descriptors = acls.all_acl_descriptors();
        let descriptor = descriptors
            .iter()
            .find(|descriptor| descriptor.contains("2179408616-0-0-0-0-1"))
            .unwrap();
        let result: Result<AdoResponse> = t.runtime.block_on(async {
            let result = t.ado.identities(&t.organization, descriptor).await?;
            assert!(!result.value.is_empty());
            Ok(result)
        });
        let _ = result.unwrap();
    }
}
