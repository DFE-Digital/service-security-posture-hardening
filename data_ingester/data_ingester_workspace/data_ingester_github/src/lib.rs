#![feature(iter_collect_into)]
//! Pull Security posture data from the Github API and send it to a Splunk HEC.
//! Uses [Octocrab] for most operations.
mod action_runs;
mod artifacts;
mod contents;
pub mod custom_properties;
pub mod entrypoint;
mod github_response;
mod org_members;
mod repos;
mod teams;
mod workflows;
use std::sync::Arc;

use crate::github_response::{GithubResponse, GithubResponses};
use crate::org_members::org_member_query;
use crate::repos::Repos;
use crate::workflows::Workflows;
use anyhow::{Context, Result};
use artifacts::{Artifact, Artifacts};
use bytes::Bytes;
use contents::Contents;
use custom_properties::{CustomProperties, CustomPropertySetter, SetOrgRepoCustomProperties};
use data_ingester_financial_business_partners::validator::Validator;
use data_ingester_sarif::{Sarif, SarifHecs};
use data_ingester_supporting::keyvault::GitHubApp;
use github_response::GithubNextLink;
use graphql_client::GraphQLQuery;
use graphql_client::Response;
use http_body_util::BodyExt;
use octocrab::models::{ArtifactId, InstallationId};
use octocrab::params::actions::ArchiveFormat;
use octocrab::Octocrab;
use org_members::{OrgMemberQuery, OrgMembers};
use serde::{Deserialize, Serialize};
use teams::GitHubTeamsOrg;
use tracing::{error, info, warn};
use workflows::{WorkflowRunJobs, WorkflowRuns};

pub static SSPHP_RUN_KEY: &str = "github";

/// NewType for Octocrab provide additonal data source.
#[derive(Debug, Clone)]
pub struct OctocrabGit {
    pub client: Octocrab,
}

impl OctocrabGit {
    pub async fn for_installation_id(&self, installation_id: InstallationId) -> Result<Self> {
        let (installation_client, _secret) =
            self.client.installation_and_token(installation_id).await?;
        Ok(Self {
            client: installation_client,
        })
    }

    pub fn new_from_app(github_app: &GitHubApp) -> Result<Self> {
        let key = jsonwebtoken::EncodingKey::from_rsa_der(&github_app.private_key); // .context("Building jsonwebtoken from gihtub app der key")?;

        // Initalise default crypto provider to prevent runtime error
        // https://github.com/rustls/rustls/issues/1938
        let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();

        let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
        let octocrab = Octocrab::builder()
            .app(github_app.app_id.into(), key)
            .build()
            .context("building Octocrab client for app")?;
        Ok(Self { client: octocrab })
    }

    /// Get a full list of [Repos] for the provided organization
    pub(crate) async fn org_repos(&self, org: &str) -> Result<Repos> {
        let mut all_repos = vec![];
        use octocrab::params::repos::Type;
        for t in [
            // Type::All,
            // Type::Forks,
            // Type::Internal,
            // Type::Member,
            Type::Private,
            Type::Public,
            // Type::Sources,
        ] {
            info!("Getting repo type: {:#?}", t);
            let page = self
                .client
                .orgs(org)
                .list_repos()
                .repo_type(Some(t))
                .per_page(100)
                .send()
                .await
                .context("getting org repos")?;
            let repos = self
                .client
                .all_pages(page)
                .await
                .context("getting additional org repo pages")?;
            info!("Got {} repos for type: {:#?}", repos.len(), t);
            all_repos.extend(repos);
        }

        all_repos.sort_by(|a, b| a.id.cmp(&b.id));
        all_repos.dedup_by(|a, b| a.id.eq(&b.id));

        Ok(Repos::new(all_repos, org))
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
    pub async fn org_teams_with_children(
        &self,
        org: &str,
    ) -> Result<(GithubResponses, GitHubTeamsOrg)> {
        let mut teams = self
            .org_teams(org)
            .await
            .context("Getting teams for {org_name}")?;

        let mut raw = vec![];
        let mut teams_org = crate::teams::GitHubTeamsOrg::new(org);

        for team in teams.responses_value_iter() {
            teams_org
                .push_team_value(team)
                .context("Adding team to org")?;

            let team_id = team
                .as_object()
                .context("Getting team as HashMap")?
                .get("id")
                .context("Getting `name` from team")?
                .as_u64()
                .context("Getting `name` as &str")?;

            info!("Getting team members for {org} {team_id}");

            let team_members = self
                .org_team_members(org, team_id)
                .await
                .context("Getting team members")?;

            teams_org
                .push_team_members_responses(team_id, &team_members)
                .context("Adding members to team&org")?;

            raw.extend(team_members.into_inner());

            let team_teams = self
                .org_team_teams(org, team_id)
                .await
                .context("Getting team members")?;

            teams_org
                .push_team_teams_responses(team_id, &team_teams)
                .context("Adding teams to org")?;

            raw.extend(team_teams.into_inner());
        }
        teams.extend(raw);
        Ok((teams, teams_org))
    }

    /// Set a custom property for an organisation
    ///
    /// `org` - The GitHub Organisation to query.
    ///
    /// `custom_property` - a `CustomProperterySetter` describing the custom property to set
    ///
    pub(crate) async fn org_create_or_update_custom_property(
        &self,
        org: &str,
        custom_property: &CustomPropertySetter,
    ) -> Result<GithubResponses> {
        let url = format!(
            "/orgs/{}/properties/schema/{}",
            org,
            custom_property.property_name()
        );

        let response = self.client._put(&url, Some(custom_property)).await?;

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

        let response_body = match serde_json::from_slice::<serde_json::Value>(&body) {
            Ok(ok) => ok,
            Err(err) => {
                let body_string = String::from_utf8(body.to_vec())
                    .unwrap_or_else(|err| format!("Unable to decode body as UTF8: {}", err));
                warn!(
                    "Error decoding create_custom_property response: {}:{} ",
                    err, body_string
                );
                anyhow::bail!(err)
            }
        };

        let github_response = GithubResponse::new(
            github_response::SingleOrVec::Single(response_body),
            url,
            status,
        );

        let github_responses = GithubResponses::from_response(github_response);

        Ok(github_responses)
    }

    pub async fn org_create_or_update_custom_property_value(
        &self,
        org: &str,
        setter: SetOrgRepoCustomProperties,
    ) -> Result<GithubResponses> {
        let url = format!("/orgs/{}/properties/values", org,);

        let response = self.client._patch(&url, Some(&setter)).await?;

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

        let response_body = match serde_json::from_slice::<serde_json::Value>(&body) {
            Ok(ok) => ok,
            Err(err) => {
                let body_string = String::from_utf8(body.to_vec())
                    .unwrap_or_else(|err| format!("Unable to decode body as UTF8: {}", err));
                warn!(
                    "Error decoding create_custom_property response: {}:{} ",
                    err, body_string
                );
                anyhow::bail!(err)
            }
        };

        let github_response = GithubResponse::new(
            github_response::SingleOrVec::Single(response_body),
            url,
            status,
        );

        let github_responses = GithubResponses::from_response(github_response);
        Ok(github_responses)
    }

    /// Get the custom properties for an organisation
    pub async fn org_get_custom_property_values(
        &self,
        org: &str,
        validator: Option<Arc<Validator>>,
    ) -> Result<CustomProperties> {
        let uri = format!("/orgs/{}/properties/values", org);
        let collection = self
            .get_collection(&uri)
            .await
            .context("Getting Custom Properties")?;
        let mut custom_properties: CustomProperties = collection.into();

        if let Some(validator) = validator {
            custom_properties
                .custom_properties
                .iter_mut()
                .for_each(|cp| cp.validate(&validator));
        }
        Ok(custom_properties)
    }

    /// Get Members for org Team
    ///
    /// `org` - The GitHub Organisation to query.
    ///
    /// `team_id`, the name / `team_id` - ID of the team. Prefer to
    /// use the numeric ID or requests can fail with non URL
    /// compatible team names
    ///
    pub(crate) async fn org_team_members<T: ToString>(
        &self,
        org: &str,
        team_id: T,
    ) -> Result<GithubResponses> {
        let uri = format!("/orgs/{org}/team/{}/members", team_id.to_string());
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
    pub(crate) async fn org_team_teams<T: ToString>(
        &self,
        org: &str,
        team_id: T,
    ) -> Result<GithubResponses> {
        let uri = format!("/orgs/{org}/team/{}/teams", team_id.to_string());
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

    /// Get Dependency Graph
    pub(crate) async fn repo_dependency_graph(&self, repo: &str) -> Result<GithubResponses> {
        let uri = format!("/repos/{repo}/dependency-graph/sbom");
        self.get_collection(&uri).await
    }

    /// Get Teams for Repo
    pub(crate) async fn repo_teams(&self, repo: &str) -> Result<GithubResponses> {
        let uri = format!("/repos/{repo}/teams");
        self.get_collection(&uri).await
    }

    /// Repo rulesets
    pub async fn repo_rulesets_full(&self, repo: &str) -> Result<GithubResponses> {
        let mut rulesets = self
            .repo_rulesets(repo)
            .await
            .context(format!("Getting Rulesets for {repo}"))?;

        let mut ruleset_details = vec![];

        for ruleset in rulesets.responses_value_iter() {
            if let Some(status) = ruleset.get("status") {
                if status == "403" {
                    let response_message = ruleset
                        .get("message")
                        .context("Getting error message from 403 response")?
                        .as_str()
                        .context("Getting 403 response as str")?;

                    warn!(
                        "error while 'Getting Rulesets for {repo}': {}",
                        response_message
                    );
                    continue;
                }
            }
            let ruleset_id = ruleset
                .get("id")
                .with_context(|| format!("Getting `id` from ruleset: {:#?}", &ruleset))?
                .as_u64()
                .context("Getting `id` as u64")?;

            info!("Getting Ruleset for {repo} {ruleset_id}");
            let repo_ruleset = self
                .repo_ruleset_by_id(repo, ruleset_id)
                .await
                .context("Getting team members")?;

            ruleset_details.extend(repo_ruleset.into_inner());
        }
        rulesets.extend(ruleset_details);
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

    /// List repository workflows
    pub(crate) async fn repo_actions_list_workflows(&self, repo: &str) -> Result<Workflows> {
        let uri = format!("/repos/{repo}/actions/workflows");
        let result = self.get_collection(&uri).await?;
        let workflows =
            Workflows::try_from(&result).context("Convert GitHubResponses to Workflows")?;
        Ok(workflows)
    }

    pub(crate) async fn repo_actions_list_workflow_runs(&self, repo: &str) -> Result<WorkflowRuns> {
        let uri = format!("/repos/{repo}/actions/runs?status=success&per_page=100");
        let result = self.get_collection(&uri).await?;
        let mut workflow_runs =
            WorkflowRuns::try_from(&result).context("Convert GitHubResponses to Workflows")?;
        workflow_runs.filter_to_lastest_runs();
        Ok(workflow_runs)
    }

    pub(crate) async fn repo_actions_list_workflow_run_jobs(
        &self,
        repo: &str,
        workflow_runs: &WorkflowRuns,
    ) -> Result<WorkflowRunJobs> {
        let mut jobs = vec![];
        for run in workflow_runs.workflow_runs.iter() {
            let uri = format!(
                "/repos/{repo}/actions/runs/{}/attempts/{}/jobs",
                run.id, run.run_attempt
            );
            let result = self.get_collection(&uri).await?;
            let workflow_run_jobs = WorkflowRunJobs::try_from(&result)?;
            jobs.extend(workflow_run_jobs.jobs);
        }
        Ok(WorkflowRunJobs {
            total_count: jobs.len(),
            jobs,
            source: "".into(),
            sourcetype: "".into(),
        })
    }

    /// Get all GitHub Actions workflow files
    pub(crate) async fn repo_actions_get_workflow_files(
        &self,
        repo: &str,
        workflows: &Workflows,
    ) -> Result<Contents> {
        let mut responses = vec![];
        for workflow in workflows.workflows.iter() {
            let uri = format!("/repos/{repo}/contents/{0}", workflow.path);
            let result = self.get_collection(&uri).await?;
            let contents =
                Contents::try_from(&result).context("Convert GitHubResponses to artifacts")?;
            responses.extend(contents.contents);
        }

        Ok(Contents {
            contents: responses,
        })
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

    /// Get thet default code scanning setup for a repo
    pub(crate) async fn repo_code_scanning_default_setup(
        &self,
        repo: &str,
    ) -> Result<GithubResponses> {
        let uri = format!("/repos/{repo}/code-scanning/default-setup");
        self.get_collection(&uri).await
    }

    /// Get 30 most recent code scanning analyses for a repo
    ///
    /// https://docs.github.com/en/rest/code-scanning/code-scanning?apiVersion=2022-11-28#list-code-scanning-analyses-for-a-repository
    pub(crate) async fn repo_code_scanning_analyses(&self, repo: &str) -> Result<GithubResponses> {
        let uri = format!("/repos/{repo}/code-scanning/analyses?per_page=1");
        self.get_single_page(&uri)
            .await
            .with_context(|| format!("Using Octocrab to get url: {}", uri))
    }

    /// Get GitHub code scanning alerts for `repo`
    pub(crate) async fn repo_code_scanning_alerts(&self, repo: &str) -> Result<GithubResponses> {
        let uri = format!("/repos/{repo}/code-scanning/alerts");
        self.get_collection(&uri).await
    }

    /// Get a list of Artifacts for a repo
    ///
    pub(crate) async fn repo_artifacts(&self, repo: &str) -> Result<Artifacts> {
        let uri = format!("/repos/{repo}/actions/artifacts");
        let result = self.get_collection(&uri).await?;
        let artifacts =
            Artifacts::try_from(&result).context("Convert GitHubResponses to artifacts")?;
        Ok(artifacts)
    }

    /// Download an artifact from Github
    ///
    /// https://medium.com/@DiggerHQ/chasing-a-nasty-bug-a-tale-of-excessive-auth-40d8bf5cf192
    pub(crate) async fn repo_artifact_download(&self, artifact: &Artifact) -> Result<Bytes> {
        let owner_name = &artifact
            .org_name()
            .context("Unable to get owner name for artifact")?;
        let repo_name = &artifact
            .repo_name()
            .context("Unable to get repo name for artifact")?;
        self.client
            .actions()
            .download_artifact(
                owner_name,
                repo_name,
                ArtifactId::from(artifact.id),
                ArchiveFormat::Zip,
            )
            .await
            .context("Getting artifact")
    }

    /// Get artifact zips from a repo where the `name` matches
    /// `filter` and convert the contained files into `SarifHecs` for
    /// submission to Splunk
    ///
    /// `repo` - The owner/repo to get the Artifacts from
    /// `filter` - A simple text patten to match against the Artifact name
    ///
    pub(crate) async fn repo_get_sarif_artifacts<'a, 'b, S1: Into<&'a str>, S2: Into<&'b str>>(
        &self,
        repo: S1,
        filter: S2,
    ) -> Result<SarifHecs> {
        let mut artifacts = self
            .repo_artifacts(repo.into())
            .await
            .expect("Getting Artifacts");

        artifacts.dedup();

        let filter = filter.into();
        let mut semgrep_artifacts = artifacts
            .artifacts
            .iter()
            .filter(|artifact| artifact.name.contains(filter))
            .collect::<Vec<&Artifact>>();

        semgrep_artifacts.dedup();

        let mut sarif_hecs = vec![];

        for artifact in semgrep_artifacts {
            let semgrep_zip = self
                .repo_artifact_download(artifact)
                .await
                .with_context(|| {
                    format!("Downloading artifact {}", artifact.archive_download_url)
                })?;
            let sarifs = Sarif::from_zip_bytes(semgrep_zip).with_context(|| {
                format!(
                    "Failed to extract Sarif file from zipfile from: {}",
                    artifact.archive_download_url
                )
            })?;
            let _ = sarifs
                .into_iter()
                .map(|sarif| sarif.to_sarif_hec(&artifact.archive_download_url, "github", "github"))
                .collect_into(&mut sarif_hecs);
        }
        Ok(SarifHecs { inner: sarif_hecs })
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
                .responses_iter()
                .any(|response| response.http_status() == 200)
            {
                responses.extend(collection.into_inner());
                break;
            }
        }
        Ok(responses.into())
    }

    /// Get a relative uri from api.github.com.
    ///
    /// Only gets the first page of results
    async fn get_single_page(&self, uri: &str) -> Result<GithubResponses> {
        let next_link = GithubNextLink::from_str(uri);

        let response = self
            .client
            ._get(next_link.next().context("no link available")?)
            .await
            .with_context(|| format!("Using Octocrab to get url: {}", uri))?;

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
                    "Error deserialising body from Github {} {}: {}",
                    uri, err, body_as_string
                );
                anyhow::bail!(err);
            }
        };

        let responses = vec![GithubResponse::new(body, uri.to_string(), status)];

        Ok(responses.into())
    }

    /// Get a relative uri from api.github.com and exhaust all next links.
    ///
    /// Returns all requests as seperate entries complete with status codes
    async fn get_collection(&self, uri: &str) -> Result<GithubResponses> {
        let mut next_link = GithubNextLink::from_str(uri);

        let mut responses = vec![];

        while let Some(next) = next_link.next() {
            let response = self
                .client
                ._get(next)
                .await
                .with_context(|| format!("Using Octocrab to get url: {}", uri))?;

            let next_next_link = GithubNextLink::from_response(&response)
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

            let body_string = std::string::String::from_utf8(body.to_vec())?;
            if status == 403 && body_string.contains("API rate limit exceeded") {
                self.wait_for_rate_limit()
                    .await
                    .context("Waiting for rate limit")?;
                continue;
            }

            next_link = next_next_link;

            let body = match serde_json::from_slice(&body).context("Deserialize body") {
                Ok(ok) => ok,
                Err(err) => {
                    let body_as_string = String::from_utf8_lossy(&body);
                    error!(
                        "Error deserialising body from Github {} {}: {}",
                        uri, err, body_as_string
                    );
                    anyhow::bail!(err);
                }
            };

            responses.push(GithubResponse::new(body, uri.to_string(), status));
        }

        Ok(responses.into())
    }

    pub(crate) async fn wait_for_rate_limit(&self) -> Result<()> {
        let rate_limit = self
            .client
            .ratelimit()
            .get()
            .await
            .context("Getting rate limit")?;
        if rate_limit.resources.core.remaining == 0 {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::SystemTime::UNIX_EPOCH)
                .context("Getting current time")?;
            let sleep_duration = tokio::time::Duration::from_secs(rate_limit.resources.core.reset)
                .saturating_sub(now);
            warn!(
                "Sleeping for {} seconds because of Core API rate limit",
                sleep_duration.as_secs()
            );
            tokio::time::sleep(sleep_duration).await;
        }

        if let Some(graphql) = rate_limit.resources.graphql {
            if graphql.remaining == 0 {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::SystemTime::UNIX_EPOCH)
                    .context("Getting current time")?;
                let sleep_duration = tokio::time::Duration::from_secs(
                    rate_limit.resources.core.reset - now.as_secs(),
                );
                warn!(
                    "Sleeping for {} seconds because of GraphQL API rate limit",
                    sleep_duration.as_secs()
                );
                tokio::time::sleep(sleep_duration).await;
            }
        }

        Ok(())
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

/// Custom DateTime struct for the GitHub API
#[derive(Clone, Default, Debug, Serialize, Deserialize, PartialOrd, Eq, PartialEq)]
struct DateTime(String);

#[cfg(feature = "live_tests")]
#[cfg(test)]
mod test {
    use std::{borrow::Borrow, env};

    use anyhow::{Context, Result};
    use data_ingester_splunk::splunk::SplunkTrait;
    use data_ingester_splunk::splunk::{Splunk, ToHecEvents};
    use data_ingester_supporting::keyvault::get_keyvault_secrets;
    use futures::future::{BoxFuture, FutureExt};
    use octocrab::models::Repository;

    use crate::{OctocrabGit, Repos};

    use tokio::sync::OnceCell;

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
                true,
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
            for repo in self.repos().iter() {
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

        fn repos(&self) -> &[Repository] {
            self.repos.repos()
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
    async fn test_github_repo_code_scanning_default_setup() -> Result<()> {
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
    async fn test_github_repo_code_scanning_analyses() -> Result<()> {
        let client = TestClient::new().await;
        client
            .repo_iter(|repo_name: &str| {
                client.client.repo_code_scanning_analyses(repo_name).boxed()
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

    /// Get a list of Artifacts for all github repos
    #[tokio::test]
    async fn test_github_repo_artifacts() -> Result<()> {
        let client = TestClient::new().await;
        client
            .repo_iter(|repo_name| client.client.repo_artifacts(repo_name).boxed())
            .await?;
        Ok(())
    }

    /// Download a sarif Artifact from GitHub
    #[tokio::test]
    async fn test_github_download_artifact() -> Result<()> {
        let client = TestClient::new().await;
        let mut total_hec_events = 0;
        for repo in client.repos().iter() {
            let repo_name = client.repo_name(&repo.name);
            let sarif_hecs = client
                .client
                .repo_get_sarif_artifacts(repo_name.as_str(), "semgrep")
                .await?;

            let hec_events = (&sarif_hecs)
                .to_hec_events()
                .context("Convert SarifHec to HecEvents")?;
            total_hec_events += hec_events.len();
            client.splunk.send_batch(hec_events).await?;
        }
        assert!(total_hec_events > 0);
        Ok(())
    }

    /// Get a list of workflows for all GitHub repos
    #[tokio::test]
    async fn test_repo_actions_list_workflows() -> Result<()> {
        let client = TestClient::new().await;
        client
            .repo_iter(|repo_name| client.client.repo_actions_list_workflows(repo_name).boxed())
            .await?;
        Ok(())
    }

    /// Get all workflow files for all GitHub repos
    #[tokio::test]
    async fn test_repo_actions_get_workflow_files() -> Result<()> {
        let client = TestClient::new().await;
        for repo in client.repos().iter() {
            let repo_name = client.repo_name(&repo.name);
            let workflows = client
                .client
                .repo_actions_list_workflows(&repo_name)
                .await?;
            let workflow_files = client
                .client
                .repo_actions_get_workflow_files(&repo_name, &workflows)
                .await?;
            let hec_events = (&workflow_files)
                .to_hec_events()
                .context("Convert SarifHec to HecEvents")?;

            client.splunk.send_batch(hec_events).await?;
        }
        Ok(())
    }

    /// Test repo dependency graph
    #[tokio::test]
    async fn test_repo_dependency_graph() -> Result<()> {
        let client = TestClient::new().await;
        for repo in client.repos().iter() {
            let repo_name = client.repo_name(&repo.name);
            let dependency_graph = client.client.repo_dependency_graph(&repo_name).await?;
            let hec_events = (&dependency_graph)
                .to_hec_events()
                .context("Convert SarifHec to HecEvents")?;

            client.splunk.send_batch(hec_events).await?;
        }
        Ok(())
    }

    #[tokio::test]
    async fn test_repo_actions_list_workflow_runs() -> Result<()> {
        let client = TestClient::new().await;
        for repo in client.repos().iter() {
            let repo_name = client.repo_name(&repo.name);
            let workflow_runs = client
                .client
                .repo_actions_list_workflow_runs(&repo_name)
                .await?;
            let hec_events = (&workflow_runs)
                .to_hec_events()
                .context("Convert workflow_runs to HecEvents")?;

            client.splunk.send_batch(hec_events).await?;
        }
        Ok(())
    }

    #[tokio::test]
    async fn test_repo_actions_list_workflow_run_jobs() -> Result<()> {
        let client = TestClient::new().await;
        for repo in client.repos().iter() {
            let repo_name = client.repo_name(&repo.name);

            let workflow_runs = client
                .client
                .repo_actions_list_workflow_runs(&repo_name)
                .await?;

            let workflow_run_jobs = client
                .client
                .repo_actions_list_workflow_run_jobs(&repo_name, &workflow_runs)
                .await?;

            let hec_events = (&workflow_run_jobs)
                .to_hec_events()
                .context("Convert SarifHec to HecEvents")?;

            client.splunk.send_batch(hec_events).await?;
        }
        Ok(())
    }
}
