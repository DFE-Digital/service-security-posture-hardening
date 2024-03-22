pub mod entrypoint;

use std::io::{Bytes, Read};

use anyhow::{Context, Result};
use data_ingester_splunk::splunk::ToHecEvents;
use data_ingester_supporting::keyvault::{GitHubApp, GitHubPat};
use http::{Response, StatusCode};
use http_body_util::combinators::{BoxBody, Collect};
use http_body_util::BodyExt;
use octocrab::models::{InstallationId, Repository};
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

    pub fn new_from_str(github_pat: &str) -> Result<Self> {
        let octocrab = Octocrab::builder()
            .personal_token(github_pat.to_string())
            .build()?;
        //        anyhow::bail!("test");
        Ok(Self { client: octocrab })
    }

    pub async fn from_installation_id(&self, installation_id: InstallationId) -> Result<Self> {
        let (installation_client, _secret) =
            self.client.installation_and_token(installation_id).await?;
        Ok(Self {
            client: installation_client,
        })
    }

    fn new_from_app(github_app: &GitHubApp) -> Result<Self> {
        let key = jsonwebtoken::EncodingKey::from_rsa_der(&github_app.private_key); // .context("Building jsonwebtoken from gihtub app der key")?;

        let octocrab = Octocrab::builder()
            .app(github_app.app_id.into(), key)
            .build()
            .context("building Octocrab client for app")?;
        Ok(Self { client: octocrab })
    }

    pub(crate) async fn org_repos(&self, org: &str) -> Result<Repos> {
        let page = self
            .client
            .orgs(org)
            .list_repos()
            .send()
            .await
            .context("getting org repos")?;
        let repos = self
            .client
            .all_pages(page)
            .await
            .context("getting additional org repo pages")?;
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

    pub(crate) async fn check_dependabot_status(&self, repo: &str) -> Result<DependabotStatus> {
        let uri = format!("/repos/{repo}/vulnerability-alerts");
        let status = self.client._get(&uri).await?;

        Ok(DependabotStatus {
            enabled: status.status() == StatusCode::from_u16(204).expect("valid status code"),
            repo: repo.to_string(),
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

    pub(crate) async fn repo_branch_protection(
        &self,
        repo: &str,
        branch: &str,
    ) -> Result<BranchProtection> {
        let uri = format!("/repos/{repo}/branches/{branch}/protection");
        let branch_protection = match self.client.get(&uri, None::<&()>).await {
            Ok(bp) => BranchProtection {
                inner: Some(bp),
                repo: repo.to_string(),
                branch_protection: true,
                branch: branch.to_string(),
            },
            Err(_) => BranchProtection {
                inner: None,
                repo: repo.to_string(),
                branch_protection: false,
                branch: branch.to_string(),
            },
        };
        Ok(branch_protection)
    }

    pub(crate) async fn codeowners(&self, repo: &str) -> Result<CodeOwners> {
        let uri = format!("/repos/{repo}/codeowners/errors");
        let response = self.client._get(&uri).await?;
        let status = response.status().as_u16();
        let body = response.collect().await?.to_bytes().slice(0..);
        let codeowners = serde_json::from_slice(&body)?;
        Ok(CodeOwners {
            inner: codeowners,
            repo: repo.to_string(),
            status: status,
        })
    }

    pub(crate) async fn org_settings(&self, org: &str) -> Result<OrgSettings> {
        let uri = format!("/orgs/{org}");
        let org_settings = self.client.get(&uri, None::<&()>).await?;
        Ok(OrgSettings {
            inner: org_settings,
            org: org.to_string(),
        })
    }

    pub(crate) async fn security_txt(&self, repo: &str) -> Result<SecurityTxt> {
        let uris = [
            format!("/repos/{repo}/contents/SECURITY.md"),
            format!("/repos/{repo}/contents/.github/SECURITY.md"),
            format!("/repos/{repo}/contents/docs/SECURITY.md"),
        ];

        let mut security_md = None;
        let mut status = 404;
        for uri in uris {
            let response = self.client._get(&uri).await?;
            status = response.status().as_u16();
            if status != 200 {
                continue;
            }
            // TODO Maybe base64 decode some fields from the body
            let body = response.collect().await?.to_bytes().slice(0..);
            security_md = Some(serde_json::from_slice(&body)?);
            break;
        }
        Ok(SecurityTxt {
            inner: security_md,
            repo: repo.to_string(),
            status: status,
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

#[derive(Serialize, Debug)]
struct DependabotStatus {
    enabled: bool,
    repo: String,
}

impl ToHecEvents for &DependabotStatus {
    type Item = Self;
    fn source(&self) -> &str {
        &self.repo
    }

    fn sourcetype(&self) -> &str {
        "github:dependabot:status"
    }

    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        Box::new(std::iter::once(self))
    }
}

#[derive(Serialize, Debug)]
struct BranchProtection {
    #[serde(flatten)]
    inner: Option<serde_json::Value>,
    repo: String,
    branch_protection: bool,
    branch: String,
}

impl ToHecEvents for &BranchProtection {
    type Item = Self;
    fn source(&self) -> &str {
        &self.repo
    }

    fn sourcetype(&self) -> &str {
        "github:branch:protection"
    }

    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        Box::new(std::iter::once(self))
    }
}

#[derive(Serialize, Debug)]
struct CodeOwners {
    #[serde(flatten)]
    inner: serde_json::Value,
    repo: String,
    status: u16,
}

impl ToHecEvents for &CodeOwners {
    type Item = Self;
    fn source(&self) -> &str {
        &self.repo
    }

    fn sourcetype(&self) -> &str {
        "github:repository:codeowners"
    }

    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        Box::new(std::iter::once(self))
    }
}

#[derive(Serialize, Debug)]
struct SecurityTxt {
    #[serde(flatten)]
    inner: Option<serde_json::Value>,
    repo: String,
    status: u16,
}

impl ToHecEvents for &SecurityTxt {
    type Item = Self;
    fn source(&self) -> &str {
        &self.repo
    }

    fn sourcetype(&self) -> &str {
        "github:repository:security_txt"
    }

    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        Box::new(std::iter::once(self))
    }
}

#[derive(Serialize, Debug)]
struct OrgSettings {
    #[serde(flatten)]
    inner: serde_json::Value,
    org: String,
}

impl ToHecEvents for &OrgSettings {
    type Item = Self;
    fn source(&self) -> &str {
        &self.org
    }

    fn sourcetype(&self) -> &str {
        "github:organisation:settings"
    }

    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        Box::new(std::iter::once(self))
    }
}
