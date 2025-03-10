use anyhow::{Context, Result};
use data_ingester_splunk::splunk::{set_ssphp_run, try_collect_send, Splunk};
use data_ingester_supporting::keyvault::Secrets;
use std::sync::Arc;
use tracing::{error, info};

use crate::{
    ado_dev_ops_client::AzureDevOpsClientMethods,
    azure_dev_ops_client_oauth::AzureDevOpsClientOauth,
    azure_dev_ops_client_pat::AzureDevOpsClientPat,
    data::{projects::Projects, repositories::Repositories},
};

pub async fn entrypoint(secrets: Arc<Secrets>, splunk: Arc<Splunk>) -> Result<()> {
    set_ssphp_run("azure_devops")?;
    info!("Starting Azure DevOps ADO collection");

    if let (Some(client_id), Some(client_secret), Some(tenant_id)) = (
        secrets.azure_client_id.as_ref(),
        secrets.azure_client_secret.as_ref(),
        secrets.azure_tenant_id.as_ref(),
    ) {
        let ado = AzureDevOpsClientOauth::new(client_id, client_secret, tenant_id)
            .await
            .context("Building AzureDevOpsClient")?;

        let organizations = try_collect_send(
            &format!("Azure DevOps Organizations for tenant: {}", tenant_id),
            ado.organizations_list(),
            &splunk,
        )
        .await?;

        for organization in organizations.organizations {
            let _collection_result =
                collect_organization(&ado, splunk.clone(), &organization.organization_name).await;
        }
    }

    for pat in secrets.ado_pats.iter() {
        if let Ok(ado) = AzureDevOpsClientPat::new(pat.organization(), pat.pat()) {
            let _ = collect_organization(&ado, splunk.clone(), pat.organization()).await;
        }
    }

    Ok(())
}

async fn collect_organization<A: AzureDevOpsClientMethods>(
    ado: &A,
    splunk: Arc<Splunk>,
    organization: &str,
) -> Result<()> {
    let _users = try_collect_send(
        &format!("Users for {organization}"),
        ado.graph_users_list(organization),
        &splunk,
    )
    .await;

    let _users = try_collect_send(
        &format!("Service Principals for {organization}"),
        ado.graph_service_principals_list(organization),
        &splunk,
    )
    .await;

    let _users = try_collect_send(
        &format!("Groups for {organization}"),
        ado.graph_groups_list(organization),
        &splunk,
    )
    .await;

    let _ = try_collect_send(
        &format!("Audit Streams for {organization}"),
        ado.audit_streams(organization),
        &splunk,
    )
    .await;

    let _ = try_collect_send(
        &format!("Advanced Security Org Enablement {organization}"),
        ado.adv_security_org_enablement(organization),
        &splunk,
    )
    .await;

    let projects = try_collect_send(
        &format!("Projects for {organization}"),
        ado.projects_list(organization),
        &splunk,
    )
    .await;

    let projects = match projects {
        Ok(projects) => Projects::from(projects),
        Err(err) => {
            error!(name="Azure Dev Ops", operation="projects_list", organization=?organization, error=?err);
            anyhow::bail!("No projects for {organization}")
        }
    };

    info!(
        name = "Azure DevOps",
        operation = "colelct_organization",
        organization = organization,
        projects_count = projects.projects.len()
    );

    let mut total_repos = 0;
    let mut active_repos = 0;

    for project in projects.projects {
        let project_name = &project.name;

        let _ = try_collect_send(
            &format!("Advanced Security Project Enablement {organization}/{project_name}"),
            ado.adv_security_project_enablement(organization, project_name),
            &splunk,
        )
        .await;

        let _ = try_collect_send(
            &format!("Policy Configuration for {organization}/{project_name}"),
            ado.policy_configuration_get(organization, project_name),
            &splunk,
        )
        .await;

        let _ = try_collect_send(
            &format!("Git Policy Configuration for {organization}/{project_name}"),
            ado.git_policy_configuration_get(organization, project_name),
            &splunk,
        )
        .await;

        let _build_genreal_settings = try_collect_send(
            &format!("Build General Settings for {organization}/{project_name}"),
            ado.build_general_settings(organization, project_name),
            &splunk,
        )
        .await;

        let repos = try_collect_send(
            &format!("Git repository list {organization}/{project_name}"),
            ado.git_repository_list(organization, project_name),
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

        info!(
            name = "Azure DevOps",
            operation = "colelct_organization",
            organization = organization,
            project = project_name,
            repo_count = repos.repositories.len()
        );

        total_repos += repos.repositories.len();
        active_repos += repos.iter_active().count();

        for repo in repos.iter_active() {
            let repo_name = &repo.name;
            let _ = try_collect_send(
                &format!(
                    "Advanced Security Repo Enablement {organization}/{project_name}/{repo_name}"
                ),
                ado.adv_security_repo_enablement(organization, project_name, repo_name),
                &splunk,
            )
            .await;

            let _ = try_collect_send(
                &format!("Advanced Security Alerts {organization}/{project_name}/{repo_name}"),
                ado.adv_security_alerts(organization, project_name, repo_name),
                &splunk,
            )
            .await;

            let _ = try_collect_send(
                &format!(
                    "Git Repo Policy Configuration for {organization}/{project_name}/{}",
                    repo.id()
                ),
                ado.git_repo_policy_configuration_get(organization, project_name, repo.id()),
                &splunk,
            )
            .await;
        }

        info!(
            name = "Azure DevOps",
            operation = "colelct_organization",
            organization = organization,
            total_repo_count = total_repos,
            active_repo_count = active_repos,
        );
    }
    Ok(())
}
