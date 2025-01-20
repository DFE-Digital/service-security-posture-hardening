use anyhow::{Context, Result};
use data_ingester_splunk::splunk::{set_ssphp_run, try_collect_send, Splunk};
use data_ingester_supporting::keyvault::Secrets;
use std::sync::Arc;
use tracing::{error, info};

use crate::{
    ado_dev_ops_client::AzureDevOpsClient,
    data::{projects::Projects, repositories::Repositories},
};

pub async fn entrypoint(secrets: Arc<Secrets>, splunk: Arc<Splunk>) -> Result<()> {
    set_ssphp_run("github")?;
    info!("Starting Azure DevOps ADO collection");

    let client_id = secrets
        .azure_client_id
        .as_ref()
        .context("No Azure Client Id")?;

    let client_secret = secrets
        .azure_client_secret
        .as_ref()
        .context("No Azure Client Secret")?;

    let tenant_id = secrets
        .azure_tenant_id
        .as_ref()
        .context("No Azure Tenant Id")?;

    let ado = AzureDevOpsClient::new(client_id, client_secret, tenant_id)
        .await
        .context("Building AzureDevOpsClient")?;

    let org_name = "foo";

    let organizations = try_collect_send(
        &format!("Azure DevOps Organizations for tenant: {}", tenant_id),
        ado.organizations_list(),
        &splunk,
    )
    .await?;

    for organization in organizations.organizations {
        let organization_name = &organization.organization_name;

        let _users = try_collect_send(
            &format!("Users for {organization_name}"),
            ado.graph_users_list(organization_name),
            &splunk,
        )
        .await;

        let _users = try_collect_send(
            &format!("Service Principals for {organization_name}"),
            ado.graph_service_principals_list(organization_name),
            &splunk,
        )
        .await;

        let _users = try_collect_send(
            &format!("Groups for {organization_name}"),
            ado.graph_groups_list(organization_name),
            &splunk,
        )
        .await;

        let _ = try_collect_send(
            &format!("Audit Streams for {organization_name}"),
            ado.audit_streams(organization_name),
            &splunk,
        )
        .await;

        let _ = try_collect_send(
            &format!("Advanced Security Org Enablement {organization_name}"),
            ado.adv_security_org_enablement(organization_name),
            &splunk,
        )
        .await;

        let projects = try_collect_send(
            &format!("Projects for {organization_name}"),
            ado.projects_list(organization_name),
            &splunk,
        )
        .await;

        let projects = match projects {
            Ok(projects) => Projects::from(projects),
            Err(err) => {
                error!(name="Azure Dev Ops", operation="projects_list", organization=?organization, error=?err);
                continue;
            }
        };

        for project in projects.projects {
            let project_name = &project.name;

            let _ = try_collect_send(
                &format!("Advanced Security Project Enablement {organization_name}/{project_name}"),
                ado.adv_security_project_enablement(organization_name, project_name),
                &splunk,
            )
            .await;

            let _ = try_collect_send(
                &format!("Policy Configuration for {organization_name}/{project_name}"),
                ado.policy_configuration_get(organization_name, project_name),
                &splunk,
            )
            .await;

            let _ = try_collect_send(
                &format!("Git Policy Configuration for {organization_name}/{project_name}"),
                ado.git_policy_configuration_get(organization_name, project_name),
                &splunk,
            )
            .await;

            let _build_genreal_settings = try_collect_send(
                &format!("Build General Settings for {org_name}/{project_name}"),
                ado.build_general_settings(organization_name, project_name),
                &splunk,
            )
            .await;

            let repos = try_collect_send(
                &format!("Git repository list {org_name}/{project_name}"),
                ado.git_repository_list(organization_name, project_name),
                &splunk,
            )
            .await;

            let repos = match repos {
                Ok(repos) => Repositories::from(repos),
                Err(err) => {
                    error!(name="Azure Dev Ops", operation="fn git_repository_list", organization=?organization, error=?err);
                    continue;
                }
            };

            for repo in repos.repositories {
                let repo_name = &repo.name;
                let _ = try_collect_send(
                    &format!("Advanced Security Repo Enablement {organization_name}/{project_name}/{repo_name}"),
                    ado.adv_security_repo_enablement(organization_name, project_name, repo_name),
                    &splunk,
                )
                    .await;

                let _ = try_collect_send(
                    &format!(
                        "Advanced Security Alerts {organization_name}/{project_name}/{repo_name}"
                    ),
                    ado.adv_security_alerts(organization_name, project_name, repo_name),
                    &splunk,
                )
                .await;
            }
        }
    }
    Ok(())
}
