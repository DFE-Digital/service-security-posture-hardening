//! Entrypoint for running the collection
use crate::{custom_properties::CustomPropertySetter, OctocrabGit};
use anyhow::{Context, Result};
use data_ingester_financial_business_partners::{fbp_results::FbpResult, validator::Validator};
use data_ingester_splunk::splunk::{set_ssphp_run, try_collect_send, Splunk, ToHecEvents};
use data_ingester_supporting::keyvault::{GitHubApp, Secrets};
use std::sync::Arc;
use tracing::{error, info};

/// Public entry point
pub async fn github_octocrab_entrypoint(secrets: Arc<Secrets>, splunk: Arc<Splunk>) -> Result<()> {
    set_ssphp_run("github")?;

    info!("Starting GitHub collection");

    info!("GIT_HASH: {}", env!("GIT_HASH"));

    if let Some(app) = secrets.github_app.as_ref() {
        let custom_property_validator = match Validator::from_splunk_fbp(secrets.clone()).await {
            Ok(validator) => Some(Arc::new(validator)),
            Err(err) => {
                error!(name="github", error=?err, "Unable to build Custom property Validator");
                None
            }
        };

        github_app(app, &splunk, custom_property_validator)
            .await
            .context("Running Collection for GitHub App")?;
    }

    Ok(())
}

/// Collect the data for a GitHub app.
///
/// Will iterate through all available Organization installations,
/// build a client for that installation and collect the posture data
/// for it.
async fn github_app(
    github_app: &GitHubApp,
    splunk: &Arc<Splunk>,
    custom_property_validator: Option<Arc<Validator>>,
) -> Result<()> {
    let client = OctocrabGit::new_from_app(github_app).context("Build OctocrabGit")?;
    info!("Getting installations");
    let installations = client
        .client
        .apps()
        .installations()
        .send()
        .await
        .context("Getting installations for github app")?;

    let mut tasks = vec![];

    for installation in installations {
        info!("Installation ID: {}", installation.id);
        if installation.account.r#type != "Organization" {
            continue;
        }
        let installation_client = client
            .for_installation_id(installation.id)
            .await
            .context("build octocrabgit client")?;
        let org_name = installation.account.login.to_string();
        info!("Installation org name: {}", &org_name);
        tasks.push((
            org_name.clone(),
            tokio::spawn(github_collect_installation_org(
                installation_client.clone(),
                org_name.clone(),
                splunk.clone(),
                custom_property_validator.clone(),
            )),
        ));
    }

    for (org_name, task) in tasks {
        task.await
            .context("Tokio task has completed successfully")?
            .with_context(|| format!("Running GitHub collection for {}", org_name))?;
    }
    Ok(())
}

async fn github_collect_installation_org(
    github_client: OctocrabGit,
    org_name: String,
    splunk: Arc<Splunk>,
    custom_property_validator: Option<Arc<Validator>>,
) -> Result<()> {
    github_client.wait_for_rate_limit().await?;
    let rate_limits = github_client.client.ratelimit().get().await?;
    let rate_limits_json = serde_json::to_string(&rate_limits)?;
    info!(name: "GitHub", org_name, rate_limits_json);

    info!("Starting collection for {}", org_name);
    let _org_settings = try_collect_send(
        &format!("Org Settings for {org_name}"),
        github_client.org_settings(&org_name),
        &splunk,
    )
    .await;

    let _org_members = try_collect_send(
        &format!("Org Members for {org_name}"),
        github_client.graphql_org_members_query(&org_name),
        &splunk,
    )
    .await;

    let org_repos = github_client
        .org_repos(&org_name)
        .await
        .context("Getting repos for {org_name}")?;
    info!(
        "Retreived {} repos for {}",
        org_repos.repos().len(),
        org_name
    );

    let events = (&org_repos)
        .to_hec_events()
        .context("Serialize GitHub repos events")?;
    splunk
        .send_batch(events)
        .await
        .context("Sending events to Splunk")?;

    let (teams, mut teams_org) = github_client
        .org_teams_with_children(&org_name)
        .await
        .context("Getting Teams for {org_name}")?;

    let teams_events = (&teams)
        .to_hec_events()
        .context("Serialize GitHub Teams and members")?;

    splunk
        .send_batch(teams_events)
        .await
        .context("Sending Github teams and members to Splunk")?;

    let team_member_events = teams_org
        .team_members_hec_events()
        .context("Creating HEC events for calculated team members")?;

    splunk
        .send_batch(&team_member_events)
        .await
        .context("Sending Calculated teams and members to Splunk")?;

    let _org_custom_properties = try_collect_send(
        &format!("Custom properties for {org_name}"),
        github_client.org_get_custom_property_values(&org_name, custom_property_validator),
        &splunk,
    )
    .await;

    for repo in org_repos.repos() {
        let rate_limits = github_client.client.ratelimit().get().await?;
        let rate_limits_json = serde_json::to_string(&rate_limits)?;
        info!(name: "GitHub", org_name, rate_limits_json);

        let repo_name = format!(
            "{}/{}",
            &repo.owner.as_ref().expect("checked owner").login,
            &repo.name
        );
        info!("Getting GitHub data for: {}", repo_name);

        let _semgrep_artifacts = try_collect_send(
            &format!("Semgrep artifacts for {repo_name}"),
            github_client.repo_get_sarif_artifacts(repo_name.as_str(), "semgrep"),
            &splunk,
        )
        .await;

        let _repo_collaborators = try_collect_send(
            &format!("Collaborators for {repo_name}"),
            github_client.repo_collaborators(&repo_name),
            &splunk,
        )
        .await;

        let _repo_teams = try_collect_send(
            &format!("Teams for {repo_name}"),
            github_client.repo_teams(&repo_name),
            &splunk,
        )
        .await;

        let _repo_code_scanning_default_setup = try_collect_send(
            &format!("Code scanning setup for {repo_name}"),
            github_client.repo_code_scanning_default_setup(&repo_name),
            &splunk,
        )
        .await;

        let _repo_code_scanning_analyses = try_collect_send(
            &format!("Code scanning analyses for {repo_name}"),
            github_client.repo_code_scanning_analyses(&repo_name),
            &splunk,
        )
        .await;

        let _code_scanning_alerts = try_collect_send(
            &format!("Code scanning alerts for {repo_name}"),
            github_client.repo_code_scanning_alerts(&repo_name),
            &splunk,
        )
        .await;

        let repo_actions_list_workflows = try_collect_send(
            &format!("Github actions list workflows for {repo_name}"),
            github_client.repo_actions_list_workflows(&repo_name),
            &splunk,
        )
        .await;

        if let Ok(repo_actions_list_workflows) = repo_actions_list_workflows {
            let _repo_actions_get_workflow_files = try_collect_send(
                &format!("GitHub actions workflow files for {repo_name}"),
                github_client
                    .repo_actions_get_workflow_files(&repo_name, &repo_actions_list_workflows),
                &splunk,
            )
            .await;
        }

        let repo_actions_list_workflow_runs = try_collect_send(
            &format!("GitHub actions workflow runs for {repo_name}"),
            github_client.repo_actions_list_workflow_runs(&repo_name),
            &splunk,
        )
        .await;

        if let Ok(repo_actions_list_workflow_runs) = repo_actions_list_workflow_runs {
            let _repo_actions_list_workflow_run_jobs = try_collect_send(
                &format!("GitHub Actions WorkflowRunJobs for {repo_name}"),
                github_client.repo_actions_list_workflow_run_jobs(
                    &repo_name,
                    &repo_actions_list_workflow_runs,
                ),
                &splunk,
            )
            .await;
        }

        let _repo_secret_scanning_alerts = try_collect_send(
            &format!("Secret Scanning Alerts for {repo_name}"),
            github_client.repo_secret_scanning_alerts(&repo_name),
            &splunk,
        )
        .await;
        let _repo_security_txt = try_collect_send(
            &format!("Security txt {repo_name}"),
            github_client.repo_security_txt(&repo_name),
            &splunk,
        )
        .await;

        let _repo_codeowners = try_collect_send(
            &format!("Codeowners for {repo_name}"),
            github_client.repo_codeowners(&repo_name),
            &splunk,
        )
        .await;

        let _repo_deploy_keys = try_collect_send(
            &format!("Deploy keys {repo_name}"),
            github_client.repo_deploy_keys(&repo_name),
            &splunk,
        )
        .await;

        let _dependabot_status = try_collect_send(
            &format!("Deploy keys {repo_name}"),
            github_client.repo_dependabot_status(&repo_name),
            &splunk,
        )
        .await;

        let _repo_dependabot_alerts = try_collect_send(
            &format!("Dependabot Alerts for {repo_name}"),
            github_client.repo_dependabot_alerts(&repo_name),
            &splunk,
        )
        .await;

        // Don't get rulesets for a repository.
        // Only get rules for the default branch
        //
        let _repo_rulesets_full = try_collect_send(
            &format!("Repo Rulesets for {repo_name}"),
            github_client.repo_rulesets_full(&repo_name),
            &splunk,
        )
        .await;

        let default_branch = match repo.default_branch.as_deref() {
            Some(default_branch) => default_branch,
            None => {
                error!("Unable to get default branch for {repo_name}");
                continue;
            }
        };

        let _repo_branch_protection = try_collect_send(
            &format!("Branch Protection for {repo_name}/{default_branch}"),
            github_client.repo_branch_protection(&repo_name, default_branch),
            &splunk,
        )
        .await;

        let _repo_branch_rules = try_collect_send(
            &format!("Rules for {repo_name}/{default_branch}"),
            github_client.repo_branch_rules(&repo_name, default_branch),
            &splunk,
        )
        .await;
    }
    Ok(())
}

pub async fn github_set_custom_properties_entrypoint(
    secrets: Arc<Secrets>,
    splunk: Arc<Splunk>,
) -> Result<()> {
    info!("Updating GitHub custom properties");

    let github_app = secrets
        .github_app
        .as_ref()
        .context("No Github App configured")?;

    let client = OctocrabGit::new_from_app(github_app).context("Build OctocrabGit")?;
    info!("Getting installations");
    let installations = client
        .client
        .apps()
        .installations()
        .send()
        .await
        .context("Getting installations for github app")?;

    let mut tasks = vec![];

    let fbp_results = Arc::new(
        FbpResult::get_results_from_splunk(secrets)
            .await
            .context("Getting FBP Results from Splunk")?,
    );

    if fbp_results.is_empty() {
        anyhow::bail!("empty fbp results");
    }

    for installation in installations {
        info!("Installation ID: {}", installation.id);

        if installation.account.r#type != "Organization" {
            continue;
        }

        let installation_client = client
            .for_installation_id(installation.id)
            .await
            .context("build octocrabgit client")?;

        let org_name = installation.account.login.to_string();

        info!("Installation org name: {}", &org_name);

        tasks.push((
            org_name.clone(),
            tokio::spawn(update_custom_properties(
                installation_client,
                org_name,
                splunk.clone(),
                fbp_results.clone(),
            )),
        ));
    }

    for (org_name, task) in tasks {
        task.await
            .context("Tokio task has completed successfully")?
            .with_context(|| format!("Running GitHub collection for {}", org_name))?;
    }

    Ok(())
}

async fn update_custom_properties(
    github_client: OctocrabGit,
    org_name: String,
    splunk: Arc<Splunk>,
    fbp_results: Arc<FbpResult>,
) -> Result<()> {
    let portfolio_setter = CustomPropertySetter::from_fbp_portfolio(fbp_results.portfolios());
    let service_line_setter =
        CustomPropertySetter::from_fbp_service_line(fbp_results.service_lines());
    let product_setter = CustomPropertySetter::from_fbp_product();

    for cps in [portfolio_setter, service_line_setter, product_setter] {
        let _repo_branch_rules = try_collect_send(
            &format!(
                "Setting GitHub Custom Property for {}/{}",
                org_name,
                cps.property_name()
            ),
            github_client.org_create_or_update_custom_property(&org_name, &cps),
            &splunk,
        )
        .await;
    }

    Ok(())
}

#[cfg(feature = "live_tests")]
#[cfg(test)]
mod live_tests {
    use std::{env, sync::Arc};

    use anyhow::{Context, Result};
    use data_ingester_financial_business_partners::ContactDetails;
    use data_ingester_splunk::splunk::Splunk;
    use data_ingester_supporting::keyvault::get_keyvault_secrets;

    use crate::entrypoint::github_octocrab_entrypoint;
    use crate::entrypoint::github_set_custom_properties_entrypoint;
    use crate::OctocrabGit;
    use data_ingester_financial_business_partners::validator::Validator;
    use tracing::error;

    #[tokio::test]
    async fn test_all_github() -> Result<()> {
        let secrets = get_keyvault_secrets(
            &env::var("KEY_VAULT_NAME").expect("Need KEY_VAULT_NAME enviornment variable"),
        )
        .await
        .unwrap();

        let splunk = Splunk::new(
            secrets.splunk_host.as_ref().context("No value")?,
            secrets.splunk_token.as_ref().context("No value")?,
        )?;

        github_octocrab_entrypoint(Arc::new(secrets), Arc::new(splunk))
            .await
            .context("Running ocotcrab full test")?;
        Ok(())
    }

    #[tokio::test]
    async fn test_github_set_custom_properties() -> Result<()> {
        let secrets = get_keyvault_secrets(
            &env::var("KEY_VAULT_NAME").expect("Need KEY_VAULT_NAME enviornment variable"),
        )
        .await
        .unwrap();

        let splunk = Splunk::new(
            secrets.splunk_host.as_ref().context("No value")?,
            secrets.splunk_token.as_ref().context("No value")?,
        )?;

        let contact_details = ContactDetails::generate_contact_details(10);

        splunk.send_into_hec_batch(&contact_details).await?;

        github_set_custom_properties_entrypoint(Arc::new(secrets), Arc::new(splunk))
            .await
            .context("Settings GitHub Custom Properties in test")?;

        Ok(())
    }

    #[tokio::test]
    async fn test_validate_custom_properties() -> Result<()> {
        let secrets = get_keyvault_secrets(
            &env::var("KEY_VAULT_NAME").expect("Need KEY_VAULT_NAME enviornment variable"),
        )
        .await
        .unwrap();
        let secrets = Arc::new(secrets);

        let custom_property_validator = match Validator::from_splunk_fbp(secrets.clone()).await {
            Ok(validator) => Some(Arc::new(validator)),
            Err(err) => {
                error!(name="github", error=?err, "Unable to build Custom property Validator");
                None
            }
        };

        let github_app = secrets.github_app.as_ref().unwrap();

        let client = OctocrabGit::new_from_app(github_app).context("Build OctocrabGit")?;

        let installations = client
            .client
            .apps()
            .installations()
            .send()
            .await
            .context("Getting installations for github app")?;

        for installation in installations {
            if installation.account.r#type != "Organization" {
                continue;
            }

            let installation_client = client
                .for_installation_id(installation.id)
                .await
                .context("build octocrabgit client")?;

            let org_name = installation.account.login.to_string();

            if org_name != "DFE-Digital" {
                continue;
            }

            let custom_properties = installation_client
                .org_get_custom_property_values(&org_name, custom_property_validator.clone())
                .await?;
            assert!(custom_properties
                .custom_properties
                .iter()
                .any(|cp| cp.validation_errors.is_none()));
        }

        Ok(())
    }
}
