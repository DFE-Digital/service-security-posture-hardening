use crate::{
    ado_dev_ops_client::AzureDevOpsClientMethods,
    ado_response::AdoLocalType,
    ado_splunk::AdoToSplunk,
    azure_dev_ops_client_oauth::AzureDevOpsClientOauth,
    azure_dev_ops_client_pat::AzureDevOpsClientPat,
    data::{
        git_policy_configuration::PolicyConfigurations, identities::Identities, organization,
        projects::Projects, repositories::Repositories, repository_policy_join::RepoPolicyJoins,
        security_acl::Acls, security_namespaces::SecurityNamespaces, stats::Stats,
    },
    SSPHP_RUN_KEY,
};
use anyhow::{Context, Result};
use data_ingester_splunk::splunk::{set_ssphp_run, try_collect_send, Splunk};
use data_ingester_splunk::splunk::{SplunkTrait, ToHecEvents};
use data_ingester_supporting::keyvault::Secrets;
use std::sync::Arc;
use tracing::{error, info, trace};

pub async fn entrypoint(secrets: Arc<Secrets>, splunk: Arc<Splunk>) -> Result<()> {
    set_ssphp_run(SSPHP_RUN_KEY)?;
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
            let ado = AzureDevOpsClientOauth::new(client_id, client_secret, tenant_id)
                .await
                .context("Building AzureDevOpsClient")?;

            let _collection_result = collect_organization(
                ado,
                splunk.clone(), //"CatsCakes").await;
                &organization.organization_name,
            )
            .await;
        }
    }

    let mut tasks = vec![];

    for pat in secrets.ado_pats.iter() {
        if let Ok(ado) = AzureDevOpsClientPat::new(pat.organization(), pat.pat()) {
            let organization = pat.organization().to_owned();
            tasks.push((
                pat.organization(),
                tokio::spawn(collect_organization(ado, splunk.clone(), organization)),
            ));
        }
    }

    for (org_name, task) in tasks {
        task.await
            .context("Tokio task has completed successfully")?
            .with_context(|| format!("Running ADO collection for {}", org_name))?;
    }

    Ok(())
}

async fn collect_organization<A: AzureDevOpsClientMethods, O: AsRef<str>>(
    ado: A,
    splunk: Arc<Splunk>,
    organization: O,
) -> Result<()> {
    let organization = organization.as_ref();
    info!(
        operation = "collect_organization",
        stage = "start",
        name = crate::SSPHP_RUN_KEY,
        ado_organization = &organization,
        "Starting ADO collection"
    );

    let _users = try_collect_send(
        &format!("Users for {organization}"),
        ado.graph_users_list(organization),
        &splunk,
    )
    .await;

    let _service_principals = try_collect_send(
        &format!("Service Principals for {organization}"),
        ado.graph_service_principals_list(organization),
        &splunk,
    )
    .await;

    let _groups = try_collect_send(
        &format!("Groups for {organization}"),
        ado.graph_groups_list(organization),
        &splunk,
    )
    .await;

    let _audit_streams = try_collect_send(
        &format!("Audit Streams for {organization}"),
        ado.audit_streams(organization),
        &splunk,
    )
    .await;

    let _adv_security = try_collect_send(
        &format!("Advanced Security Org Enablement {organization}"),
        ado.adv_security_org_enablement(organization),
        &splunk,
    )
    .await;

    trace!(
        name = crate::SSPHP_RUN_KEY,
        ado_organization = &organization,
        "collect_security_namespaces"
    );
    let security_namespaces = try_collect_send(
        &format!("Security Namespaces {organization}"),
        ado.security_namespaces(organization),
        &splunk,
    )
    .await;

    trace!(
        name = crate::SSPHP_RUN_KEY,
        ado_organization = &organization,
        "security_namespaces_outer"
    );
    // Get Acls & Identities associated with all Security Namespaces
    if let Ok(security_namespaces) = security_namespaces {
        trace!(
            name = crate::SSPHP_RUN_KEY,
            ado_organization = &organization,
            "security_namespaces_inner"
        );
        let security_namespaces = SecurityNamespaces::from(security_namespaces);

        trace!(
            name = crate::SSPHP_RUN_KEY,
            ado_organization = &organization,
            "collect_security_acls"
        );
        let security_access_control_lists =
            collect_security_acls(&ado, &splunk, security_namespaces, organization).await;

        trace!(
            name = crate::SSPHP_RUN_KEY,
            ado_organization = &organization,
            "all_acl_descriptors"
        );
        let identity_descriptors = security_access_control_lists.all_acl_descriptors();

        trace!(
            name = crate::SSPHP_RUN_KEY,
            ado_organization = &organization,
            "collect_identities"
        );
        let _ = collect_identities(&ado, &splunk, identity_descriptors, organization).await;
    }
    trace!(
        name = crate::SSPHP_RUN_KEY,
        ado_organization = &organization,
        "projects_list"
    );
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

    trace!(
        name = "Azure DevOps",
        operation = "colelct_organization",
        organization = organization,
        projects_count = projects.projects.len()
    );

    let mut total_repos = 0;
    let mut active_repos = 0;

    for project in &projects.projects {
        let project_id = &project.id;

        let _ = try_collect_send(
            &format!("Advanced Security Project Enablement {organization}/{project_id}"),
            ado.adv_security_project_enablement(organization, project),
            &splunk,
        )
        .await;

        let policies = try_collect_send(
            &format!("Policy Configuration for {organization}/{project_id}"),
            ado.policy_configuration_get(organization, project),
            &splunk,
        )
        .await;

        let _policy_types = try_collect_send(
            &format!("Policy Configuration for {organization}/{project_id}"),
            ado.policy_types_get(organization, project),
            &splunk,
        )
        .await;

        let policies = match policies {
            Ok(policies) => PolicyConfigurations::from((policies, project.id.as_str())),
            Err(err) => {
                error!(name="Azure Dev Ops", operation="fn policy_configuration_get", organization=?organization, error=?err);
                continue;
            }
        };

        let _ = try_collect_send(
            &format!("Git Policy Configuration for {organization}/{project_id}"),
            ado.git_policy_configuration_get(organization, project),
            &splunk,
        )
        .await;

        let _build_genreal_settings = try_collect_send(
            &format!("Build General Settings for {organization}/{project_id}"),
            ado.build_general_settings(organization, project),
            &splunk,
        )
        .await;

        let mut repos = {
            let repos = ado.git_repository_list(organization, project).await;

            let repos = match repos {
                Ok(response) => response,
                Err(err) => {
                    error!(name="Azure Dev Ops", operation="fn git_repository_list", organization=?organization, error=?err);
                    continue;
                }
            };
            Repositories::from(repos)
        };

        {
            let repo_policy_joins =
                RepoPolicyJoins::from_repo_policies(organization, project, &repos, &policies);

            let repo_policy_joins_hec_events = match repo_policy_joins.to_hec_events() {
                Ok(hec_events) => hec_events,
                Err(err) => {
                    error!(name="Azure Dev Ops", operation="RepoPolicyJoins::from_repo_policies", organization=?organization, error=?err);
                    vec![]
                }
            };

            let _ = splunk.send_batch(repo_policy_joins_hec_events).await;
        }

        trace!(
            name = "Azure DevOps",
            operation = "collect_organization",
            organization = organization,
            project = project_id,
            repo_count = repos.repositories.len()
        );

        total_repos += repos.repositories.len();
        active_repos += repos.iter_active().count();

        for repo in repos.repositories.iter_mut() {
            let repo_id = &repo.id();
            let _ = try_collect_send(
                &format!("Advanced Security Repo Enablement {organization}/{project_id}/{repo_id}"),
                ado.adv_security_repo_enablement(organization, project, repo),
                &splunk,
            )
            .await;

            let _ = try_collect_send(
                &format!("Advanced Security Alerts {organization}/{project_id}/{repo_id}"),
                ado.adv_security_alerts(organization, project, repo),
                &splunk,
            )
            .await;

            let repo_stats_name = &format!("Repo stats list {organization}/{project_id}/{repo_id}");
            let stats = 'stats: {
                let stats = try_collect_send(
                    repo_stats_name,
                    ado.repo_stats_list(organization, project, repo),
                    &splunk,
                )
                .await;

                let stats = match stats {
                    Ok(stats) => stats,
                    Err(err) => {
                        error!(name="Azure Dev Ops", operation="fn git_repository_list", organization=?organization, error=?err);
                        break 'stats None;
                    }
                };
                let stats: Stats = AdoLocalType::from(stats).into_inner();
                Some(stats)
            };

            if let Some(stats) = stats {
                repo.add_most_recent_stat(stats);
            }

            AdoToSplunk::from_metadata(&repos.metadata)
                .event(&repo)
                .send(&splunk, &repo_stats_name)
                .await?;
        }

        info!(
            name = crate::SSPHP_RUN_KEY,
            operation = "collect_organization",
            stage = "end",
            organization = organization,
            total_repo_count = total_repos,
            active_repo_count = active_repos,
        );
    }
    Ok(())
}

/// Get the associated ACLs for `SecurityNamespaces`
///
async fn collect_security_acls(
    ado: &impl AzureDevOpsClientMethods,
    splunk: &Splunk,
    security_namespaces: SecurityNamespaces,
    organization: &str,
) -> Acls {
    let mut security_access_control_lists = Acls::default();

    // Get ACLS
    for namespace in &security_namespaces.namespaces {
        let namespace_id = namespace.namespace_id.as_str();
        let name = &format!("Security Access control lists {organization}/{namespace_id}");
        let security_access_control_list = try_collect_send(
            &name,
            ado.security_access_control_lists(organization, namespace_id),
            splunk,
        )
        .await;

        // Process Acls for Splunk.
        // The aces_dictionary format is arduous to work with in Splunk so we convert to an aces_vec
        if let Ok(mut security_access_control_list) = security_access_control_list {
            let metadata = std::mem::take(&mut security_access_control_list.metadata);
            let acls: Acls = security_access_control_list.into();

            let _ = AdoToSplunk::from_metadata(&metadata)
                .events(&acls.inner)
                .send(splunk, &name)
                .await;

            security_access_control_lists.extend(acls);
        }
    }
    security_access_control_lists
}

/// Collect all identites from an  Iterator of &str.
///
/// There are cases where the the ADO API returns a different identity
/// descriptor than the one actually requested. This seems to be
/// limited to project level administraive groups and is probably due
/// to an underlying ADO implementation detail. In these cases we send
/// the returned / mismatched version of the identity, and then modify
/// that idetity to contain the requested descriptor and send that
/// too. This will allow any usecases in Splunk to match against
/// either of the desciptors.
async fn collect_identities(
    ado: &impl AzureDevOpsClientMethods,
    splunk: &Splunk,
    identity_descriptors: impl IntoIterator<Item = &str>,
    organization: &str,
) {
    for descriptor in identity_descriptors {
        let name = &format!("User identities {organization} {}", descriptor);
        let user_identities =
            try_collect_send(name, ado.identities(organization, descriptor), splunk).await;
        if let Ok(user_identities) = user_identities {
            let metadata = user_identities.metadata.clone();
            let mut identities: Identities = user_identities.into();
            for id in identities.inner.iter_mut() {
                if id.descriptor != descriptor {
                    id.descriptor = descriptor.to_string();
                    let _ = AdoToSplunk::from_metadata(&metadata)
                        .event(id)
                        .send(splunk, name)
                        .await;
                }
            }
        }
    }
}
