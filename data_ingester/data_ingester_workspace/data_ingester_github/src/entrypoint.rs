use std::sync::Arc;

use anyhow::{Context, Result};
use data_ingester_splunk::splunk::{set_ssphp_run, try_collect_send, Splunk, ToHecEvents};
use data_ingester_supporting::keyvault::{GitHubPat, Secrets};

use crate::OctocrabGit;

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

    match &secrets.github {
        data_ingester_supporting::keyvault::GitHub::App(_app) => todo!(),
        data_ingester_supporting::keyvault::GitHub::Pat(pat) => github_pat(pat, &splunk).await?,
        data_ingester_supporting::keyvault::GitHub::None => {
            splunk
                .log("No GitHub Secrets")
                .await
                .context("Failed logging to Splunk")?;
            anyhow::bail!("No github application secrets");
        }
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
            &format!("Dependabot Alerts for {repo_name}"),
            github_client.repo_dependabot_alerts(&repo_name),
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
