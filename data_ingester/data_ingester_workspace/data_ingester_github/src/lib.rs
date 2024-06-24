//! Pull Security posture data from the Github API and send it to a Splunk HEC.
//! Uses [Octocrab] for most operations.
pub mod entrypoint;
use anyhow::{Context, Result};
use data_ingester_splunk::splunk::ToHecEvents;
use data_ingester_supporting::keyvault::GitHubApp;
use graphql_client::{GraphQLQuery, Response};
use http_body_util::BodyExt;
use itertools::Itertools;
use octocrab::models::{InstallationId, Repository};
use octocrab::Octocrab;
use regex::Regex;
use serde::{Deserialize, Serialize};
use tokio::sync::OnceCell;
use tracing::{error, info};

/// NewType for Octocrab provide additonal data source.
#[derive(Clone)]
pub(crate) struct OctocrabGit {
    client: Octocrab,
}

impl OctocrabGit {
    pub async fn for_installation_id(&self, installation_id: InstallationId) -> Result<Self> {
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

    /// Get a full list of [Repos] for the provided organization
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

    /// Get the settings for the org
    pub(crate) async fn org_settings(&self, org: &str) -> Result<GithubResponses> {
        let uri = format!("/orgs/{org}");
        self.get_collection(&uri).await
    }

    /// Get Teams for org
    pub(crate) async fn org_teams(&self, org: &str) -> Result<GithubResponses> {
        let uri = format!("/orgs/{org}/teams");
        self.get_collection(&uri).await
    }

    /// Get the Teams for an org with child objects
    ///
    /// This will get the members for each team and the child teams for each team
    ///
    pub async fn org_teams_with_chilren(&self, org: &str) -> Result<GithubResponses> {
        let mut teams = self
            .org_teams(org)
            .await
            .context("Getting teams for {org_name}")?;

        let mut members = vec![];
        let mut team_teams = vec![];

        for team in teams.inner.iter().flat_map(|ghr| match &ghr.response {
            crate::SingleOrVec::Vec(ref vec) => vec.to_vec(),
            crate::SingleOrVec::Single(single) => vec![single.clone()],
        }) {
            let team_name = team
                .as_object()
                .context("Getting team as HashMap")?
                .get("id")
                .context("Getting `name` from team")?
                .as_u64()
                .context("Getting `name` as &str")?;

            info!("Getting team members for {org} {team_name}");
            members.extend(
                self.org_team_members(org, team_name)
                    .await
                    .context("Getting team members")?
                    .inner,
            );
            team_teams.extend(
                self.org_team_teams(org, team_name)
                    .await
                    .context("Getting team members")?
                    .inner,
            );
        }
        teams.inner.extend(members);
        teams.inner.extend(team_teams);
        Ok(teams)
    }

    /// Get Members for org Team
    ///
    /// `org` - The GitHub Organisation to query.
    ///
    /// `team_id`, the name / `team_id` - ID of the team. Prefer to
    /// use the numeric ID or requests can fail with non URL
    /// compatible team names
    ///
    pub(crate) async fn org_team_member<T :ToString>(&self, org: &str, team_id: T) -> Result<GithubResponses> {
        let uri = format!("/orgs/{org}/teams/{}/members", team_id.to_string());
        dbg!(&uri);
        self.get_collection(&uri).await
    }

    /// Get Members for org Team
    ///
    /// `org` - The GitHub Organisation to query.
    ///
    ///`team_id`, the name / `team_id` - ID of the team. Prefer to use
    /// the numeric ID or requests can fail with non URL compatible
    /// team names
    ///
    pub(crate) async fn org_team_teams<T: ToString>(&self, org: &str, team_id: T) -> Result<GithubResponses> {
        let uri = format!("/orgs/{org}/teams/{}/teams", team_id.to_string());
        self.get_collection(&uri).await
    }

    /// Get branch protection for a specific repo & branch
    pub(crate) async fn repo_branch_protection(
        &self,
        repo: &str,
        branch: &str,
    ) -> Result<GithubResponses> {
        let uri = format!("/repos/{repo}/branches/{branch}/protection");
        self.get_collection(&uri).await
    }

    /// Get Collaborators for Repo
    pub(crate) async fn repo_collaborators(&self, repo: &str) -> Result<GithubResponses> {
        let uri = format!("/repos/{repo}/collaborators");
        self.get_collection(&uri).await
    }

    /// Get Collaborators for Repo
    pub(crate) async fn repo_teams(&self, repo: &str) -> Result<GithubResponses> {
        let uri = format!("/repos/{repo}/teams");
        self.get_collection(&uri).await
    }

    /// Repo rulesets
    pub async fn repo_rulesets_full(&self, repo: &str) -> Result<GithubResponses> {
        let mut rulesets = self
            .repo_rulesets(repo)
            .await
            .context("Getting Rulesets for {repo}")?;

        let mut ruleset_details = vec![];

        for ruleset in rulesets.inner.iter().flat_map(|ghr| match &ghr.response {
            crate::SingleOrVec::Vec(ref vec) => vec.to_vec(),
            crate::SingleOrVec::Single(single) => vec![single.clone()],
        }) {
            let ruleset_id = ruleset
                .get("id")
                .context("Getting `id` from team")?
                .as_u64()
                .context("Getting `id` as u64")?;

            info!("Getting Ruleset for {repo} {ruleset_id}");
            ruleset_details.extend(
                self.repo_ruleset_by_id(repo, ruleset_id)
                    .await
                    .context("Getting team members")?
                    .inner,
            );
        }
        rulesets.inner.extend(ruleset_details);
        Ok(rulesets)
    }

    /// Get GitHub Rulesets for repo
    pub(crate) async fn repo_rulesets(&self, repo: &str) -> Result<GithubResponses> {
        let uri = format!("/repos/{repo}/rulesets");
        self.get_collection(&uri).await
    }

    /// Get the details for a specific ruleset
    ///
    /// Doesn't seem to get the 'bypass_actors' property listed at
    /// https://docs.github.com/en/rest/repos/rules?apiVersion=2022-11-28#get-a-repository-ruleset
    ///
    pub(crate) async fn repo_ruleset_by_id(
        &self,
        repo: &str,
        ruleset_id: u64,
    ) -> Result<GithubResponses> {
        let uri = format!("/repos/{repo}/rulesets/{ruleset_id}");
        self.get_collection(&uri).await
    }

    /// Get GitHub Rules for a specific repo & branch
    pub(crate) async fn repo_branch_rules(
        &self,
        repo: &str,
        branch: &str,
    ) -> Result<GithubResponses> {
        let uri = format!("/repos/{repo}/rules/branches/{branch}");
        self.get_collection(&uri).await
    }

    /// Get Dependabot alerts for a repo
    pub(crate) async fn repo_dependabot_alerts(&self, repo: &str) -> Result<GithubResponses> {
        let uri = format!("/repos/{repo}/dependabot/alerts");
        self.get_collection(&uri).await
    }

    /// Get the dependabot status for a repo
    pub(crate) async fn repo_dependabot_status(&self, repo: &str) -> Result<GithubResponses> {
        let uri = format!("/repos/{repo}/vulnerability-alerts");
        self.get_collection(&uri).await
    }

    /// Get deploy keys for a repo
    pub(crate) async fn repo_deploy_keys(&self, repo: &str) -> Result<GithubResponses> {
        let uri = format!("/repos/{repo}/keys");
        self.get_collection(&uri).await
    }

    pub(crate) async fn repo_code_scanning_default_setup(
        &self,
        repo: &str,
    ) -> Result<GithubResponses> {
        let uri = format!("/repos/{repo}/code-scanning/default-setup");
        self.get_collection(&uri).await
    }

    pub(crate) async fn repo_codeowners(&self, repo: &str) -> Result<GithubResponses> {
        let uri = format!("/repos/{repo}/codeowners/errors");
        self.get_collection(&uri).await
    }

    pub(crate) async fn repo_secret_scanning_alerts(&self, repo: &str) -> Result<GithubResponses> {
        let uri = format!("/repos/{repo}/secret-scanning/alerts");
        self.get_collection(&uri).await
    }

    pub(crate) async fn repo_security_txt(&self, repo: &str) -> Result<GithubResponses> {
        let uris = [
            format!("/repos/{repo}/contents/SECURITY.md"),
            format!("/repos/{repo}/contents/.github/SECURITY.md"),
            format!("/repos/{repo}/contents/docs/SECURITY.md"),
        ];
        let mut responses = vec![];
        for uri in uris {
            let collection = self.get_collection(&uri).await?;
            if collection
                .inner
                .iter()
                .any(|response| response.ssphp_http_status == 200)
            {
                responses.extend(collection.inner);
                break;
            }
        }
        Ok(GithubResponses { inner: responses })
    }

    /// Get a relative uri from api.github.com and exhaust all next links.
    ///
    /// Returns all requests as seperate entries complete with status codes
    async fn get_collection(&self, uri: &str) -> Result<GithubResponses> {
        let mut next_link = GithubNextLink::from_str(uri);

        let mut responses = vec![];

        while let Some(next) = next_link.next {
            let response = self.client._get(next).await.context("Get url")?;

            next_link = GithubNextLink::from_response(&response)
                .await
                .context("Failed getting response 'link'")?;

            let status = response.status().as_u16();
            let mut body = response
                .collect()
                .await
                .context("collect body")?
                .to_bytes()
                .slice(0..);

            if body.is_empty() {
                body = "{}".into();
            }

            let body = match serde_json::from_slice(&body).context("Deserialize body") {
                Ok(ok) => ok,
                Err(err) => {
                    let body_as_string = String::from_utf8_lossy(&body);
                    error!(
                        "Error deserialising body from Github {}: {}",
                        err, body_as_string
                    );
                    anyhow::bail!(err);
                }
            };

            responses.push(GithubResponse {
                response: body,
                source: uri.to_string(),
                ssphp_http_status: status,
            });
        }

        Ok(GithubResponses { inner: responses })
    }

    /// Get an Organizations list of members with their roles(ADMIN/MEMBER) from graph QL
    ///
    /// org_name: The name of the Organisation to get the members for
    pub(crate) async fn graphql_org_members_query(&self, org_name: &str) -> Result<OrgMembers> {
        let mut variables = org_member_query::Variables {
            login: org_name.to_string(),
            members_after: None,
            members_first: 100,
        };
        let mut next_page = true;
        let mut org_members = OrgMembers::default();
        while next_page {
            let query = OrgMemberQuery::build_query(variables.clone());
            let response: Response<org_member_query::ResponseData> =
                self.client.graphql(&query).await?;
            let organisation = &response
                .data
                .as_ref()
                .context("data")?
                .organization
                .as_ref()
                .context("org")?;
            let members_page_info = &organisation.members_with_role.page_info;
            next_page = members_page_info.has_next_page;
            variables
                .members_after
                .clone_from(&members_page_info.end_cursor);
            org_members
                .extend(response)
                .context("add response to members")?;
        }
        Ok(org_members)
    }
}

/// Containing for OrgMembers
#[derive(Default, Debug)]
struct OrgMembers(Vec<OrgMember>);

impl OrgMembers {
    /// Extend the members from a [org_member_query::ResponseData]
    pub(crate) fn extend(
        &mut self,
        response: Response<org_member_query::ResponseData>,
    ) -> Result<()> {
        let members: OrgMembers = response.try_into().context("Convert to OrgMembers")?;
        self.0.extend(members.0);
        Ok(())
    }
}

/// Hec Event descriptor for Org Members
impl ToHecEvents for &OrgMembers {
    type Item = OrgMember;

    fn source(&self) -> &str {
        "graphql:org_members_query"
    }

    fn sourcetype(&self) -> &str {
        "github"
    }

    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        Box::new(self.0.iter())
    }
    fn ssphp_run_key(&self) -> &str {
        "github"
    }
}

/// Org member representation sent to Splunk
#[derive(Serialize, Default, Debug)]
struct OrgMember {
    organisation: String,
    login: String,
    email: String,
    role: org_member_query::OrganizationMemberRole,
}

/// Convert a [org_member_query::ResponseData] to [OrgMembers]
impl TryFrom<Response<org_member_query::ResponseData>> for OrgMembers {
    type Error = anyhow::Error;
    fn try_from(value: Response<org_member_query::ResponseData>) -> Result<Self> {
        let organization = value
            .data
            .and_then(|data| data.organization)
            .context("OrgQueryOrganization should be present")?;
        let organization_login = organization.login.as_str();
        let members_with_roles = organization
            .members_with_role
            .edges
            .context("Expect member_with_role edges to be present")?;

        let mut org_members = Vec::with_capacity(members_with_roles.len());

        for member_with_role in members_with_roles.iter().flatten() {
            let role = member_with_role
                .role
                .as_ref()
                .context("Role to be present")?;
            let member = member_with_role
                .node
                .as_ref()
                .context("Expect member to be present")?;
            let org_member = OrgMember {
                organisation: organization_login.to_owned(),
                login: member.login.to_owned(),
                email: member.email.to_owned(),
                role: role.clone(),
            };
            org_members.push(org_member);
        }
        Ok(OrgMembers(org_members))
    }
}

impl Default for org_member_query::OrganizationMemberRole {
    fn default() -> Self {
        Self::MEMBER
    }
}

/// Autogenerated structs from 'src/org_member_query.graphql'
#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/schema.docs.graphql",
    query_path = "src/org_member_query.graphql",
    response_derives = "Clone, Debug, Default",
    variables_derives = "Clone, Debug"
)]
pub struct OrgMemberQuery;

/// Custom DateTime struct for the GitHub API
#[derive(Clone, Default, Debug, Serialize, Deserialize)]
struct DateTime(String);

#[cfg(test)]
mod test {
    use std::{borrow::Borrow, env};

    use anyhow::{Context, Result};
    use data_ingester_splunk::splunk::{Splunk, ToHecEvents};
    use data_ingester_supporting::keyvault::get_keyvault_secrets;
    use futures::future::{BoxFuture, FutureExt};

    use crate::{OctocrabGit, Repos};

    use tokio::sync::OnceCell;

    #[derive(Clone)]
    struct TestClient {
        client: OctocrabGit,
        org_name: String,
        repos: Repos,
        splunk: Splunk,
    }
    static CLIENT: OnceCell<TestClient> = OnceCell::const_new();

    impl TestClient {
        async fn new() -> &'static Self {
            CLIENT
                .get_or_init(|| async {
                    TestClient::setup_app()
                        .await
                        .expect("Github Test Client should complete setup")
                })
                .await
        }

        async fn setup_app() -> Result<TestClient> {
            let secrets = get_keyvault_secrets(
                &env::var("KEY_VAULT_NAME").expect("Need KEY_VAULT_NAME enviornment variable"),
            )
            .await
            .unwrap();
            let splunk = Splunk::new(
                secrets.splunk_host.as_ref().context("No value")?,
                secrets.splunk_token.as_ref().context("No value")?,
            )?;

            let github_app = secrets
                .github_app
                .as_ref()
                .expect("Github App should be configured");
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

                let org_repos = installation_client
                    .org_repos(org_name)
                    .await
                    .context("Getting repos for org")?;
                return Ok(TestClient {
                    client: installation_client,
                    org_name: org_name.to_string(),
                    repos: org_repos,
                    splunk,
                });
            }
            anyhow::bail!("no github client created");
        }

        fn repo_name(&self, repo_name: &str) -> String {
            format!("{}/{}", &self.org_name, &repo_name)
        }

        async fn repo_iter<F, T, H>(&self, func: F) -> Result<()>
        where
            F: FnOnce(&str) -> BoxFuture<'_, Result<T>> + Copy,
            T: std::fmt::Debug + Borrow<H>,
            for<'a> &'a H: ToHecEvents,
        {
            for repo in self.repos.inner.iter() {
                let repo_name = self.repo_name(&repo.name);
                let result = func(&repo_name).await.unwrap();

                let events = result
                    .borrow()
                    .to_hec_events()
                    .context("ToHecEvents Serialize")?;

                self.splunk
                    .send_batch(events)
                    .await
                    .context("Sending events to Splunk")?;
            }
            Ok(())
        }

        async fn org<F, T, H>(&self, func: F) -> Result<()>
        where
            F: FnOnce(&str) -> BoxFuture<'_, Result<T>> + Copy,
            T: std::fmt::Debug + Borrow<H>,
            for<'a> &'a H: ToHecEvents,
        {
            let result = func(&self.org_name).await.unwrap();

            let events = result
                .borrow()
                .to_hec_events()
                .context("ToHecEvents Serialize")?;

            self.splunk
                .send_batch(events)
                .await
                .context("Sending events to Splunk")?;
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_github_repo_code_scanning() -> Result<()> {
        let client = TestClient::new().await;
        client
            .repo_iter(|repo_name: &str| {
                client
                    .client
                    .repo_code_scanning_default_setup(repo_name)
                    .boxed()
            })
            .await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_github_repo_codeowners() -> Result<()> {
        let client = TestClient::new().await;
        client
            .repo_iter(|repo_name: &str| client.client.repo_codeowners(repo_name).boxed())
            .await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_github_repo_deploy_keys() -> Result<()> {
        let client = TestClient::new().await;
        client
            .repo_iter(|repo_name: &str| client.client.repo_deploy_keys(repo_name).boxed())
            .await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_github_repo_dependabot_status() -> Result<()> {
        let client = TestClient::new().await;
        client
            .repo_iter(|repo_name: &str| client.client.repo_dependabot_status(repo_name).boxed())
            .await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_github_repo_dependabot_alerts() -> Result<()> {
        let client = TestClient::new().await;
        client
            .repo_iter(|repo_name: &str| client.client.repo_dependabot_alerts(repo_name).boxed())
            .await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_github_repo_secret_scanning() -> Result<()> {
        let client = TestClient::new().await;
        client
            .repo_iter(|repo_name: &str| {
                client.client.repo_secret_scanning_alerts(repo_name).boxed()
            })
            .await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_github_org_settings() -> Result<()> {
        let client = TestClient::new().await;
        client
            .org(|org_name: &str| client.client.org_settings(org_name).boxed())
            .await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_github_graphql() -> Result<()> {
        let client = TestClient::new().await;
        let _result = client.client.graphql_org_members_query("403ind").await?;
        Ok(())
    }
}

#[derive(Serialize, Debug, Clone)]
struct Repos {
    inner: Vec<Repository>,
    source: String,
}

/// New type for Vec<[Repository]> including the source of the repository
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
        "github"
    }

    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        Box::new(self.inner.iter())
    }
    fn ssphp_run_key(&self) -> &str {
        "github"
    }
}

/// A collection of API responses from Github
#[derive(Serialize, Debug)]
struct GithubResponses {
    inner: Vec<GithubResponse>,
}

impl ToHecEvents for &GithubResponses {
    type Item = GithubResponse;

    /// Not used
    fn source(&self) -> &str {
        unimplemented!()
    }

    /// Not used
    fn sourcetype(&self) -> &str {
        unimplemented!()
    }

    /// Not used
    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        unimplemented!()
    }

    /// Create a collection of
    /// [data_ingester_splunk::splunk::HecEvent] for each element in
    /// of a Github response, in a collection of github responses.
    fn to_hec_events(&self) -> Result<Vec<data_ingester_splunk::splunk::HecEvent>> {
        Ok(self
            .inner
            .iter()
            .flat_map(|response| response.to_hec_events())
            .flatten()
            .collect())
    }
    fn ssphp_run_key(&self) -> &str {
        "github"
    }
}

/// An  API responses from Github
#[derive(Serialize, Debug)]
struct GithubResponse {
    #[serde(flatten)]
    response: SingleOrVec,
    #[serde(skip)]
    source: String,
    ssphp_http_status: u16,
}

/// Descriminator type to help [serde::Deserialize] deal with API endpoints that return a '{}' or a '[{}]'
#[derive(Deserialize, Serialize, Debug)]
#[serde(untagged)]
enum SingleOrVec {
    Vec(Vec<serde_json::Value>),
    Single(serde_json::Value),
}

impl ToHecEvents for &GithubResponse {
    type Item = Self;
    fn source(&self) -> &str {
        &self.source
    }

    fn sourcetype(&self) -> &str {
        "github"
    }

    /// Not used
    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        unimplemented!()
    }

    /// Create a [data_ingester_splunk::splunk::HecEvent] for each
    /// element of a collection returned by a single GitHub api call.
    fn to_hec_events(&self) -> Result<Vec<data_ingester_splunk::splunk::HecEvent>> {
        // TODO FIX THIS
        // Shouldn't have to clone all the values :(
        let data = match &self.response {
            SingleOrVec::Single(single) => vec![single.clone()],
            SingleOrVec::Vec(vec) => vec.to_vec(),
        };

        let (ok, _err): (Vec<_>, Vec<_>) = data
            .iter()
            .map(|event| GithubResponse {
                response: SingleOrVec::Single(event.clone()),
                source: self.source.clone(),
                ssphp_http_status: self.ssphp_http_status,
            })
            .map(|gr| {
                data_ingester_splunk::splunk::HecEvent::new_with_ssphp_run(
                    &gr,
                    self.source(),
                    self.sourcetype(),
                    self.get_ssphp_run(),
                )
            })
            .partition_result();
        Ok(ok)
    }

    fn ssphp_run_key(&self) -> &str {
        "github"
    }
}

/// Helper for paginating GitHub resoponses.
///
/// Represents the link to the next page of results for a paginated Github request.
///
/// The link is stored as just the path and query elements of the URI
/// for compatibility with OctoCrab authentication
///
#[derive(Debug)]
struct GithubNextLink {
    next: Option<String>,
}

impl GithubNextLink {
    /// Use the exact url as the next link
    fn from_str(url: impl Into<String>) -> Self {
        Self {
            next: Some(url.into()),
        }
    }

    /// Take a `link` header, as returned  by Github, and create a new [GithubNextLink] from it.
    async fn from_link_str(header: &str) -> Self {
        static CELL: OnceCell<Regex> = OnceCell::const_new();
        let regex = CELL
            .get_or_init(|| async {
                Regex::new(r#"<(?<url>[^>]+)>; rel=\"next\""#).expect("Regex is valid")
            })
            .await;

        let next = regex
            .captures(header)
            .and_then(|cap| cap.name("url").map(|m| m.as_str().to_string()))
            .and_then(|url| http::uri::Uri::from_maybe_shared(url).ok())
            .and_then(|uri| uri.path_and_query().map(|pq| pq.as_str().to_string()));

        Self { next }
    }

    /// Create a next link from a [http::Response] from GitHub API.
    async fn from_response<T>(response: &http::Response<T>) -> Result<Self> {
        let header = if let Some(header) = response.headers().get("link") {
            header
                .to_str()
                .context("Unable to parse GitHub link header")?
        } else {
            return Ok(Self { next: None });
        };
        Ok(Self::from_link_str(header).await)
    }
}

#[cfg(test)]
mod test_github_next_link {
    use anyhow::Result;

    use crate::GithubNextLink;
    #[tokio::test]
    async fn test_github_links() -> Result<()> {
        let header = "<https://api.github.com/repositories/123456789/dependabot/alerts?per_page=1&page=2>; rel=\"next\", <https://api.github.com/repositories/123456789/dependabot/alerts?per_page=1&page=5>; rel=\"last\"";

        let next = GithubNextLink::from_link_str(header).await;
        assert!(next.next.is_some());
        assert_eq!(
            next.next.unwrap(),
            "/repositories/123456789/dependabot/alerts?per_page=1&page=2".to_string()
        );
        Ok(())
    }
}
