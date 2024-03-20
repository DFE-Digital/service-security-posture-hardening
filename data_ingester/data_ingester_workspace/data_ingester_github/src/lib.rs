pub mod entrypoint;

use anyhow::{Context, Result};
use data_ingester_splunk::splunk::ToHecEvents;
use data_ingester_supporting::keyvault::{GitHubApp, GitHubPat};
use octocrab::models::Repository;
use octocrab::{Octocrab, Page};
// use octorust::types::{
//     MinimalRepository, Order, RateLimitOverview, ReposListOrgSort, ReposListOrgType, Repository, SimpleUser
// };
use http::uri::Uri;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

#[derive(Clone)]
pub struct OctocrabGit {
    client: Octocrab,
}

impl OctocrabGit {
    pub fn new_from_pat(github_pat: &GitHubPat) -> Result<Self> {
        let octocrab = Octocrab::builder()
            .personal_token(github_pat.pat.to_string())
            .build()?;
        //        anyhow::bail!("test");
        Ok(Self { client: octocrab })
    }

    pub(crate) async fn org_repos(&self, org: &str) -> Result<Repos> {
        let page = self.client.orgs(org).list_repos().send().await?;
        let repos = self.client.all_pages(page).await?;
        Ok(Repos::new(repos, org))
    }

    pub(crate) async fn org_dependabot_alerts(&self, org: &str) -> Result<DependabotAlerts> {
        let uri = format!("/orgs/{org}/dependabot/alerts");
        let alerts = self.get_all_pages(&uri).await?;
        Ok(DependabotAlerts {
            inner: alerts,
            source: org.to_string(),
        })
    }

    /// Repo should be in the form of "owner/repo"
    pub(crate) async fn repo_dependabot_alerts(&self, repo: &str) -> Result<DependabotAlerts> {
        let uri = format!("/repos/{repo}/dependabot/alerts");
        let alerts = self.get_all_pages(&uri).await?;
        Ok(DependabotAlerts {
            inner: alerts,
            source: repo.to_string(),
        })
    }

    async fn get_all_pages<T: Serialize + DeserializeOwned>(&self, uri: &str) -> Result<Vec<T>> {
        let url = Uri::builder()
            .path_and_query(uri)
            .build()
            .expect("valid uri");
        let page: Page<T> = self
            .client
            .get_page(&Some(url))
            .await?
            .context("Need at least 1 page of alerts")?;
        let all = self.client.all_pages(page).await?;
        Ok(all)
    }

    pub(crate) async fn repos_for_current_pat(&self) -> Result<Repos> {
        let page = self
            .client
            .current()
            .list_repos_for_authenticated_user()
            .per_page(100)
            .type_("owner")
            .send()
            .await?;
        let repos = self.client.all_pages(page).await?;
        let user = self.client.current().user().await?;
        Ok(Repos::new(repos, &user.login))
    }
}

#[derive(Serialize, Debug)]
struct Repos {
    inner: Vec<Repository>,
    source: String,
}

impl Repos {
    fn new(repos: Vec<Repository>, org: &str) -> Self {
        Self {
            inner: repos,
            source: format!("github:{}", org),
        }
    }
}

impl ToHecEvents for &Repos {
    type Item = Repository;
    fn source(&self) -> &str {
        &self.source
    }

    fn sourcetype(&self) -> &str {
        "github:repository"
    }

    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        Box::new(self.inner.iter())
    }
}

#[derive(Serialize, Debug)]
struct DependabotAlerts {
    inner: Vec<DependabotAlert>,
    source: String,
}

impl DependabotAlerts {
    fn new(repos: Vec<DependabotAlert>, source: &str) -> Self {
        Self {
            inner: repos,
            source: source.to_string(),
        }
    }
}

impl ToHecEvents for &DependabotAlerts {
    type Item = DependabotAlert;
    fn source(&self) -> &str {
        &self.source
    }

    fn sourcetype(&self) -> &str {
        "github:dependabot:alert"
    }

    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        Box::new(self.inner.iter())
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct DependabotAlert {
    #[serde(flatten)]
    inner: serde_json::Value,
}
