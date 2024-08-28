//! Entrypoint for running the collection
use std::sync::Arc;

use crate::OctocrabGit;
use anyhow::{Context, Result};
use data_ingester_splunk::splunk::{set_ssphp_run, try_collect_send, Splunk, ToHecEvents};
use data_ingester_supporting::keyvault::{GitHubApp, Secrets};
use tracing::{error, info};

/// Public entry point
pub async fn github_octocrab_entrypoint(secrets: Arc<Secrets>, splunk: Arc<Splunk>) -> Result<()> {
    set_ssphp_run("github")?;

    info!("Starting GitHub collection");

    info!("GIT_HASH: {}", env!("GIT_HASH"));

    if let Some(app) = secrets.github_app.as_ref() {
        github_app(app, &splunk)
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
async fn github_app(github_app: &GitHubApp, splunk: &Arc<Splunk>) -> Result<()> {
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
        tasks.push(tokio::spawn(github_collect_installation_org(
            installation_client,
            org_name,
            splunk.clone(),
        )));
    }

    for task in tasks {
        let _ = task
            .await
            .context("Running GitHub collection for all installations")?;
    }
    Ok(())
}

async fn github_collect_installation_org(
    github_client: OctocrabGit,
    org_name: String,
    splunk: Arc<Splunk>,
) -> Result<()> {
    // DO NOT MERGE TO MAIN
    if org_name != "DFE-Digital" {
        return Ok(());
    }
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
        &format!("Org Settings for {org_name}"),
        github_client.graphql_org_members_query(&org_name),
        &splunk,
    )
    .await;

    let org_repos = github_client
        .org_repos(&org_name)
        .await
        .context("Getting repos for {org_name}")?;
    info!("Retreived {} repos for {}", org_repos.inner.len(), org_name);

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

    dbg!(org_repos.inner.len());

    for repo in org_repos.inner {
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
            &format!("Getting Semgrep artifacts for {repo_name}"),
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
            &format!("Code scanning alerts for {repo_name}"),
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
            &format!("GitHub actions workflow files for {repo_name}"),
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

#[cfg(feature = "live_tests")]
#[cfg(test)]
mod test {
    use std::{env, sync::Arc};

    use anyhow::{Context, Result};
    use data_ingester_splunk::splunk::Splunk;
    use data_ingester_supporting::keyvault::get_keyvault_secrets;

    use crate::entrypoint::github_octocrab_entrypoint;

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
}
