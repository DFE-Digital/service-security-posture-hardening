use std::sync::Arc;

use crate::OctocrabGit;
use anyhow::{Context, Result};
use data_ingester_splunk::splunk::{set_ssphp_run, try_collect_send, Splunk, ToHecEvents};
use data_ingester_supporting::keyvault::{GitHubApp, GitHubPat, Secrets};
use octocrab::{models::InstallationToken, params::apps::CreateInstallationAccessToken};
use secrecy::{CloneableSecret, DebugSecret, ExposeSecret, Secret, Zeroize};

pub async fn github_octocrab(secrets: Arc<Secrets>, splunk: Arc<Splunk>) -> Result<()> {
    set_ssphp_run()?;

    splunk
        .log("Starting GitHub collection")
        .await
        .context("Failed logging to Splunk")?;
    splunk
        .log(&format!("GIT_HASH: {}", env!("GIT_HASH")))
        .await
        .context("Failed logging to Splunk")
        .context("Failed logging to Splunk")?;

    // if let Some(app) = secrets.github.app.as_ref() {
    //     github_app(app, &splunk).await?;
    // }

    if let Some(pat) = secrets.github.pat.as_ref() {
        github_pat(pat, &splunk).await?;
    }

    // match &secrets.github {
    //     data_ingester_supporting::keyvault::GitHub::App(app) => github_app(app, &splunk).await?,
    //     data_ingester_supporting::keyvault::GitHub::Pat(pat) => github_pat(pat, &splunk).await?,
    //     data_ingester_supporting::keyvault::GitHub::None => {
    //         splunk
    //             .log("No GitHub Secrets")
    //             .await
    //             .context("Failed logging to Splunk")?;
    //         anyhow::bail!("No github application secrets");
    //     }
    // }

    Ok(())
}

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
            .from_installation_id(installation.id)
            .await
            .context("build octocrabgit client")?;
        let org_name = &installation.account.login.to_string();
        github_collect_installation_org(&installation_client, &org_name, splunk)
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
            &format!("Security txt {repo_name}"),
            github_client.security_txt(&repo_name),
            splunk,
        )
        .await?;

        try_collect_send(
            &format!("Codeowners for {repo_name}"),
            github_client.codeowners(&repo_name),
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
            .check_dependabot_status(&repo_name)
            .await
            .context("getting dependabot status")?;

        let events = (&dependabot_status)
            .to_hec_events()
            .context("Serialize dependabot status events")?;
        splunk
            .send_batch(events)
            .await
            .context("Sending events to Splunk")?;

        if !dependabot_status.enabled {
            continue;
        }

        try_collect_send(
            &format!("Dependabot Alerts for {repo_name}"),
            github_client.repo_dependabot_alerts(&repo_name),
            splunk,
        )
        .await?;
    }
    Ok(())
}

async fn github_pat(github_pat: &GitHubPat, splunk: &Arc<Splunk>) -> Result<()> {
    let client =
        OctocrabGit::new_from_pat(github_pat).context("Building OctocabGit client from pat")?;
    dbg!(client
        .client
        .ratelimit()
        .get()
        .await
        .context("Getting rate limit")?);
    for org_name in &github_pat.orgs {
        github_collect_pat_org(&client, org_name, splunk)
            .await
            .context("Github collection for pat/org")?;
    }
    Ok(())
}

async fn github_collect_pat_org(
    github_client: &OctocrabGit,
    org_name: &str,
    splunk: &Arc<Splunk>,
) -> Result<()> {
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
        // Ignore repos with no owner
        if repo.owner.is_none() {
            continue;
        }

        // Skip repos we can't access
        if let Some(permissions) = repo.permissions.as_ref() {
            if !permissions.admin || !permissions.maintain || !permissions.push {
                continue;
            }
        }

        let repo_name = format!(
            "{}/{}",
            &repo.owner.as_ref().expect("checked owner").login,
            &repo.name
        );

        try_collect_send(
            &format!("Security txt {repo_name}"),
            github_client.security_txt(&repo_name),
            splunk,
        )
        .await?;

        try_collect_send(
            &format!("Codeowners for {repo_name}"),
            github_client.codeowners(&repo_name),
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
            .check_dependabot_status(&repo_name)
            .await
            .context("getting dependabot status")?;
        let events = (&dependabot_status)
            .to_hec_events()
            .context("Serialize dependabot status events")?;
        splunk
            .send_batch(events)
            .await
            .context("Sending events to Splunk")?;

        if dependabot_status.enabled {
            try_collect_send(
                &format!("Dependabot Alerts for {repo_name}"),
                github_client.repo_dependabot_alerts(&repo_name),
                splunk,
            )
            .await?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod test {
    use std::{env, sync::Arc};

    use anyhow::{Context, Result};
    use data_ingester_splunk::splunk::Splunk;
    use data_ingester_supporting::keyvault::get_keyvault_secrets;

    use crate::entrypoint::github_octocrab;

    //    use super::github;
    #[tokio::test]
    async fn test_github() -> Result<()> {
        let secrets = get_keyvault_secrets(
            &env::var("KEY_VAULT_NAME").expect("Need KEY_VAULT_NAME enviornment variable"),
        )
        .await
        .unwrap();
        let splunk = Splunk::new(&secrets.splunk_host, &secrets.splunk_token)
            .context("building Splunk client")?;

        github_octocrab(Arc::new(secrets), Arc::new(splunk))
            .await
            .context("Running ocotcrab full test")?;
        Ok(())
    }
}

// async fn github_app(github_app: &GitHubApp, splunk: &Arc<Splunk>) -> Result<()> {
//     let app = GitHub::new_from_app(&github_app)?;
//     let installations = app.client.apps().list_all_installations(None, "").await?.body;
//     for installation in &installations {
//         if installation.target_type != "Organization" {
//             continue
//         }

//         let github_client = GitHub::new_from_installation(installation.id, &github_app)?;
//         let org_name = &installation.account.simple_user.login;
//         github_collect(&github_client, org_name, splunk).await?;
//     }
//     Ok(())
// }
