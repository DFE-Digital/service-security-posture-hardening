//! Entrypoint for running the collection
use std::sync::Arc;

use crate::OctocrabGit;
use anyhow::{Context, Result};
use data_ingester_splunk::splunk::{set_ssphp_run, try_collect_send, Splunk, ToHecEvents};
use data_ingester_supporting::keyvault::{GitHubApp, Secrets};
use tracing::info;

/// Public entry point
pub async fn github_octocrab_entrypoint(secrets: Arc<Secrets>, splunk: Arc<Splunk>) -> Result<()> {
    set_ssphp_run()?;

    info!("Starting GitHub collection");

    info!("GIT_HASH: {}", env!("GIT_HASH"));

    if let Some(app) = secrets.github_app.as_ref() {
        github_app(app, &splunk).await?;
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
        let org_name = &installation.account.login.to_string();
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
    try_collect_send(
        &format!("Org Settings for {org_name}"),
        github_client.org_settings(org_name),
        splunk,
    )
    .await?;

    let org_repos = github_client
        .org_repos(org_name)
        .await
        .context("Getting repos for org")?;

    let events = (&org_repos)
        .to_hec_events()
        .context("Serialize ResourceGraphResponse.data events")?;
    splunk
        .send_batch(events)
        .await
        .context("Sending events to Splunk")?;

    for repo in org_repos.inner {
        let repo_name = format!(
            "{}/{}",
            &repo.owner.as_ref().expect("checked owner").login,
            &repo.name
        );

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

        try_collect_send(
            &format!("Branch Protection for {repo_name}"),
            github_client.repo_branch_protection(
                &repo_name,
                &repo.default_branch.unwrap_or_else(|| "main".to_string()),
            ),
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
    }
    Ok(())
}

// async fn github_pat(github_pat: &GitHubPat, splunk: &Arc<Splunk>) -> Result<()> {
//     let client =
//         OctocrabGit::new_from_pat(github_pat).context("Building OctocabGit client from pat")?;
//     dbg!(client
//         .client
//         .ratelimit()
//         .get()
//         .await
//         .context("Getting rate limit")?);
//     for org_name in &github_pat.orgs {
//         github_collect_pat_org(&client, org_name, splunk)
//             .await
//             .context("Github collection for pat/org")?;
//     }
//     Ok(())
// }

// async fn github_collect_pat_org(
//     github_client: &OctocrabGit,
//     org_name: &str,
//     splunk: &Arc<Splunk>,
// ) -> Result<()> {
//     let org_repos = github_client
//         .org_repos(org_name)
//         .await
//         .context("Getting repos for org")?;

//     let events = (&org_repos)
//         .to_hec_events()
//         .context("Serialize ResourceGraphResponse.data events")?;
//     splunk
//         .send_batch(events)
//         .await
//         .context("Sending events to Splunk")?;

//     for repo in org_repos.inner {
//         // Ignore repos with no owner
//         if repo.owner.is_none() {
//             continue;
//         }

//         // Skip repos we can't access
//         if let Some(permissions) = repo.permissions.as_ref() {
//             if !permissions.admin || !permissions.maintain || !permissions.push {
//                 continue;
//             }
//         }

//         let repo_name = format!(
//             "{}/{}",
//             &repo.owner.as_ref().expect("checked owner").login,
//             &repo.name
//         );

//         try_collect_send(
//             &format!("Deploy keys {repo_name}"),
//             github_client.repo_deploy_keys(&repo_name),
//             splunk,
//         )
//         .await?;

//         try_collect_send(
//             &format!("Security txt {repo_name}"),
//             github_client.repo_security_txt(&repo_name),
//             splunk,
//         )
//         .await?;

//         try_collect_send(
//             &format!("Codeowners for {repo_name}"),
//             github_client.repo_codeowners(&repo_name),
//             splunk,
//         )
//         .await?;

//         try_collect_send(
//             &format!("Branch Protection for {repo_name}"),
//             github_client.repo_branch_protection(
//                 &repo_name,
//                 &repo.default_branch.unwrap_or_else(|| "main".to_string()),
//             ),
//             splunk,
//         )
//         .await?;

//         let dependabot_status = github_client
//             .repo_dependabot_status(&repo_name)
//             .await
//             .context("getting dependabot status")?;
//         let events = (&dependabot_status)
//             .to_hec_events()
//             .context("Serialize dependabot status events")?;
//         splunk
//             .send_batch(events)
//             .await
//             .context("Sending events to Splunk")?;

//         try_collect_send(
//             &format!("Dependabot Alerts for {repo_name}"),
//             github_client.repo_dependabot_alerts(&repo_name),
//             splunk,
//         )
//         .await?;
//     }
//     Ok(())
// }

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
        let splunk = Splunk::new(&secrets.splunk_host, &secrets.splunk_token)
            .context("building Splunk client")?;

        github_octocrab_entrypoint(Arc::new(secrets), Arc::new(splunk))
            .await
            .context("Running ocotcrab full test")?;
        Ok(())
    }
}
