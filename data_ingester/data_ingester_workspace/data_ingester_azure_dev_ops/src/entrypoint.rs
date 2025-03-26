use crate::{
    ado_dev_ops_client::AzureDevOpsClientMethods,
    ado_response::AdoLocalType,
    azure_dev_ops_client_oauth::AzureDevOpsClientOauth,
    azure_dev_ops_client_pat::AzureDevOpsClientPat,
    data::{
        git_policy_configuration::PolicyConfigurations, graph_users::{User,Users}, identities::{self, Identity}, projects::Projects, repositories::{AdoToHecEvent, Repositories}, repository_policy_join::RepoPolicyJoins, security_acl::{Acl, Acls}, security_namespaces::SecurityNamespaces, stats::Stats
    },
    SSPHP_RUN_KEY,
};
use anyhow::{Context, Result};
use data_ingester_splunk::splunk::ToHecEvents;
use data_ingester_splunk::splunk::{set_ssphp_run, try_collect_send, Splunk};
use data_ingester_supporting::keyvault::Secrets;
use std::sync::Arc;
use tracing::{error, info};
use std::collections::HashMap;

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
            let _collection_result =
                collect_organization(&ado, splunk.clone(), //"CatsCakes").await;
                                      &organization.organization_name).await;
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
    // let users = try_collect_send(
    //     &format!("Users for {organization}"),
    //     ado.graph_users_list(organization),
    //     &splunk,
    // )
    //     .await;

    // if let Ok(users) =  users {
    //     let users: Vec<User> = users.value.into_iter().map(|value| serde_json::from_value(value).unwrap()).collect();
    //     for user in &users {
    //         let _user_identities = try_collect_send(
    //             &format!("User identities {organization} {}", user.display_name),
    //             ado.identities(organization, &user.descriptor),
    //             &splunk,
    //         )
    //             .await;
    //     }
    // } else {
    //     let err = users.unwrap_err();
    //     error!(name="Azure Dev Ops", operation="security_namespaces", organization=?organization, error=?err);
    // };
    
    // anyhow::bail!("finish here");    
    
    


    // let _users = try_collect_send(
    //     &format!("Service Principals for {organization}"),
    //     ado.graph_service_principals_list(organization),
    //     &splunk,
    // )
    // .await;

    // let _users = try_collect_send(
    //     &format!("Groups for {organization}"),
    //     ado.graph_groups_list(organization),
    //     &splunk,
    // )
    // .await;



    // let _ = try_collect_send(
    //     &format!("Audit Streams for {organization}"),
    //     ado.audit_streams(organization),
    //     &splunk,
    // )
    // .await;

    // let _ = try_collect_send(
    //     &format!("Advanced Security Org Enablement {organization}"),
    //     ado.adv_security_org_enablement(organization),
    //     &splunk,
    // )
    // .await;

    let security_namespaces = try_collect_send(
        &format!("Security Namespaces {organization}"),
        ado.security_namespaces(organization),
        &splunk,
    )
        .await;

    let mut identities: HashMap<String, Identity> = HashMap::new();    

    if let Ok(security_namespaces) =  security_namespaces {
        let security_namespaces = SecurityNamespaces::from(security_namespaces);
        for namespace in &security_namespaces.namespaces {
            
            let security_access_control_lists = try_collect_send(
                &format!("Security Namespaces {organization}"),
                ado.security_access_control_lists(organization, namespace.namespace_id.as_str()),
                &splunk,
            )
                .await;
            
            if let Ok(acls) = security_access_control_lists {
                let acls_: Vec<Acl> = acls.value.into_iter().map(|value| serde_json::from_value(value).unwrap()).collect();
                for acl in &acls_ {
                    for user in acl.aces_dictionary.keys() {
                        if identities.contains_key(user) {
                            println!("hashmap contains user: {}", &user); 
                            continue
                        }
                        
                        let user_identities = try_collect_send(
                            &format!("User identities {organization} {}", user),
                            ado.identities(organization, &user),
                            &splunk,
                        )
                            .await;
                        if let Ok(user_identities) = user_identities {
                            let user_identities
                                : Vec<Identity> = user_identities.value.into_iter().filter(|value| {
                                    if value.is_null() {
                                        error!("identity {} IS NULL!", user);
                                        false
                                    } else {
                                        true
                                    }
                                    }).map(|value| serde_json::from_value(value).unwrap()).collect();
                            for identity in user_identities {
                                identities.insert(user.to_string(), identity);
                            }
                        }
                        
                        
                    }
             
                }
                let mut acls = Acls {
                    acls: acls_,
                    metadata: acls.metadata
                };
                acls.prepare_for_splunk();
                let hec_events= acls.to_hec_events().unwrap();
                splunk.send_batch(hec_events).await;
            } 

            
        }
    } else {
        let err = security_namespaces.unwrap_err();
        error!(name="Azure Dev Ops", operation="security_namespaces", organization=?organization, error=?err);
    };


    
    // if let Ok(users) =  users {
    //     let users: Vec<User> = users.value.into_iter().map(|value| serde_json::from_value(value).unwrap()).collect();
    //     for user in &users {
    //         let _user_identities = try_collect_send(
    //             &format!("User identities {organization} {}", user.display_name),
    //             ado.identities(organization, &user.descriptor),
    //             &splunk,
    //         )
    //             .await;
    //     }
    // } else {
    //     let err = users.unwrap_err();
    //     error!(name="Azure Dev Ops", operation="security_namespaces", organization=?organization, error=?err);
    // };
    
//    anyhow::bail!("finish here");            
    

    

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

        info!(
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

            let stats = 'stats: {
                let stats = try_collect_send(
                    &format!("Repo stats list {organization}/{project_id}/{repo_id}"),
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

            let repo_hec_event = AdoToHecEvent {
                inner: &repo,
                metadata: &repos.metadata,
            }
            .to_hec_events();

            match repo_hec_event {
                Ok(event) => {
                    let _ = splunk.send_batch(event).await;
                }
                Err(err) => {
                    error!(error=?err, repo=?repo, "Failed to Convert ADO Repo to HecEvent");
                }
            }
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
