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
    for installation in installations {
        info!("Installation ID: {}", installation.id);
        if installation.account.r#type != "Organization" {
            continue;
        }
        let installation_client = client
            .for_installation_id(installation.id)
            .await
            .context("build octocrabgit client")?;
        let org_name = &installation.account.login.to_string();
        info!("Installation org name: {}", &org_name);
        github_collect_installation_org(&installation_client, org_name, splunk)
            .await
            .context("Collect data for installation")?;
    }
    Ok(())
}

async fn github_collect_installation_org(
    github_client: &OctocrabGit,
    org_name: &str,
    splunk: &Arc<Splunk>,
) -> Result<()> {
    info!("Starting collection for {}", org_name);
    try_collect_send(
        &format!("Org Settings for {org_name}"),
        github_client.org_settings(org_name),
        splunk,
    )
    .await?;

    try_collect_send(
        &format!("Org Settings for {org_name}"),
        github_client.graphql_org_members_query(org_name),
        splunk,
    )
    .await?;

    let org_repos = github_client
        .org_repos(org_name)
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

    try_collect_send(
        &format!("Getting Teams for {org_name}"),
        github_client.org_teams_with_chilren(org_name),
        splunk,
    )
    .await?;

    for repo in org_repos.inner {
        let repo_name = format!(
            "{}/{}",
            &repo.owner.as_ref().expect("checked owner").login,
            &repo.name
        );

        try_collect_send(
            &format!("Collaborators for {repo_name}"),
            github_client.repo_collaborators(&repo_name),
            splunk,
        )
        .await?;

        try_collect_send(
            &format!("Teams for {repo_name}"),
            github_client.repo_teams(&repo_name),
            splunk,
        )
        .await?;

        try_collect_send(
            &format!("Code scanning for {repo_name}"),
            github_client.repo_code_scanning_default_setup(&repo_name),
            splunk,
        )
        .await?;

        try_collect_send(
            &format!("Secret Scanning Alerts for {repo_name}"),
            github_client.repo_secret_scanning_alerts(&repo_name),
            splunk,
        )
        .await?;

        try_collect_send(
            &format!("Security txt {repo_name}"),
            github_client.repo_security_txt(&repo_name),
            splunk,
        )
        .await?;

        try_collect_send(
            &format!("Codeowners for {repo_name}"),
            github_client.repo_codeowners(&repo_name),
            splunk,
        )
        .await?;

        try_collect_send(
            &format!("Deploy keys {repo_name}"),
            github_client.repo_deploy_keys(&repo_name),
            splunk,
        )
        .await?;

        let dependabot_status = github_client
            .repo_dependabot_status(&repo_name)
            .await
            .context("getting dependabot status")?;

        let events = (&dependabot_status)
            .to_hec_events()
            .context("Serialize dependabot status events")?;

        splunk
            .send_batch(events)
            .await
            .context("Sending events to Splunk")?;

        try_collect_send(
            &format!("Dependabot Alerts for {repo_name}"),
            github_client.repo_dependabot_alerts(&repo_name),
            splunk,
        )
        .await?;

        // Don't get rulesets for a repository.
        // Only get rules for the default branch
        //
        try_collect_send(
            &format!("Repo Rulesets for {repo_name}"),
            github_client.repo_rulesets_full(&repo_name),
            splunk,
        )
        .await?;

        let default_branch = match repo.default_branch.as_deref() {
            Some(default_branch) => default_branch,
            None => {
                error!("Unable to get default branch for {repo_name}");
                continue;
            }
        };

        try_collect_send(
            &format!("Branch Protection for {repo_name}/{default_branch}"),
            github_client.repo_branch_protection(&repo_name, default_branch),
            splunk,
        )
        .await?;

        try_collect_send(
            &format!("Rules for {repo_name}/{default_branch}"),
            github_client.repo_branch_rules(&repo_name, default_branch),
            splunk,
        )
        .await?;
    }
    Ok(())
}

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
