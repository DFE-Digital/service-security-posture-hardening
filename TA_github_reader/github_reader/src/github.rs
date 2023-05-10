use anyhow::{Context, Result};
use futures::join;
use modular_input::{Event, Input, ModularInput, Scheme};
use octorust::types::{MinimalRepository, Order, ReposListOrgSort, ReposListOrgType};
use octorust::{auth::Credentials, Client};
use std::fmt;
use std::sync::mpsc::Sender;
use tracing::instrument;
use tracing::{debug, info};

#[derive(Clone)]
pub struct GitHub {
    client: Client,
}

impl fmt::Debug for GitHub {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Github")
    }
}

impl GitHub {
    //#[instrument]
    // Create a new GitHub Client from a PAT
    pub async fn new(token: &str) -> Self {
        let client = octorust::Client::new(
            String::from("reporeader"),
            Credentials::Token(String::from(token)),
        )
        .unwrap();
        GitHub { client }
    }

    // Confirm github_token and org combo work by accessing the Github API
    pub async fn check_access(&self, org: &str) -> Result<()> {
        self.client.orgs().get(org).await?;
        Ok(())
    }

    //#[instrument]
    // Collect all repos for an `org` and and send them to the Splunk Event writer
    pub async fn repos(
        &self,
        org: &str,
        template_event: &Event,
        event_writer: Sender<Event>,
    ) -> Result<Vec<MinimalRepository>> {
        debug!("github");
        let mut count = 1;
        let mut repositories = vec![];
        loop {
            info!("getting repos");
            let repos = self
                .client
                .repos()
                .list_for_org(
                    org,
                    ReposListOrgType::All,
                    ReposListOrgSort::FullName,
                    Order::Asc,
                    100,
                    count,
                )
                .await?;
            info!("repos: {}", repos.len());
            if repos.is_empty() {
                break;
            }
            count += 1;
            info!("writing repos");
            for repo in repos.iter() {
                let cloned_event = template_event.clone();

                let new_event = cloned_event.data_from_ssphp_run(&repo)?;
                event_writer.send(new_event)?;
            }
            repositories.extend(repos);
        }
        Ok(repositories)
    }

    #[instrument]
    // Collect all members for an `org` and and send them to the Splunk Event writer
    pub async fn members(
        &self,
        org: &str,
        template_event: &Event,
        event_writer: Sender<Event>,
    ) -> Result<()> {
        debug!("github members");
        let mut count = 1;
        loop {
            let members = self
                .client
                .orgs()
                .list_members(
                    org,
                    octorust::types::OrgsListMembersFilter::All,
                    octorust::types::OrgsListMembersRole::All,
                    100,
                    count,
                )
                .await?;
            info!("members: {}", members.len());
            if members.is_empty() {
                break;
            }
            count += 1;
            for member in members {
                let cloned_event = template_event.clone();
                let new_event = cloned_event.data_from_ssphp_run(&member)?;

                event_writer.send(new_event)?;
            }
        }
        Ok(())
    }

    #[instrument]
    // Get rate limit stats for the current token and send them to Splunk
    pub async fn rate_limit(
        &self,
        template_event: &Event,
        event_writer: Sender<Event>,
    ) -> Result<()> {
        let rate_limit = self.client.rate_limit().get().await?;
        let new_event = template_event.clone().data_from_ssphp_run(&rate_limit)?;

        event_writer.send(new_event)?;
        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct GitHubMI {
    pub org: String,
    pub github_token: String,
}

// impl<'a> ModularInput for Foo<'a> {
impl ModularInput for GitHubMI {
    fn from_input(input: &Input) -> Result<Self> {
        Ok(GitHubMI {
            org: input
                .param_by_name("org")
                .context("Missing `org` parameter!")?
                .to_string(),
            github_token: input
                .param_by_name("github_token")
                .context("Missing `github_token` parameter!")?
                .to_string(),
        })
    }

    //#[instrument]
    async fn run(&self) -> Result<()> {
        let time = self.current_time()?;

        let (event_writer_thread, event_writer) = self.start_event_writing_thread();

        let github = GitHub::new(&self.github_token).await;
        let source = format!("github_{}", &self.org);
        let template_event = Event::new()
            .source(&source)
            .sourcetype("github_repo_json")
            .time(time);
        let gh1 = github.clone();
        let repos = gh1.repos(&self.org, &template_event, event_writer.clone());

        let template_event = Event::new()
            .source(&source)
            .sourcetype("github_members_json")
            .time(time);
        let gh2 = github.clone();
        let members = gh2.members(&self.org, &template_event, event_writer.clone());
        let template_event = Event::new()
            .source(&source)
            .sourcetype("github_rate_limit")
            .time(time);

        let gh3 = github.clone();
        let rate_limit = gh3.rate_limit(&template_event, event_writer.clone());

        let (repo_r, members_r, rate_limit_r) = join!(repos, members, rate_limit);
        repo_r.context("Failed to get repositories")?;
        members_r.context("Failed to get members")?;
        rate_limit_r.context("Failed to get rate limit")?;
        std::mem::drop(event_writer);
        event_writer_thread.join().unwrap()?;
        Ok(())
    }

    async fn validate_arguments(&self) -> Result<()> {
        GitHub::new(&self.github_token)
            .await
            .check_access(&self.org)
            .await
            .context(format!(
                "Unable to access Org:{} with token:{}...",
                &self.org,
                &self.github_token.chars().take(8).collect::<String>()
            ))?;
        info!("Arguments valid!");
        Ok(())
    }

    #[instrument]
    fn scheme() -> Result<()> {
        let scheme = Scheme {
            title: "SSPHP_GitHub".to_string(),
            description: "SSPHP GitHub reader - for repos, members, & teams ".to_string(),
            streaming_mode: "xml".to_string(),
            use_single_instance: false,
        };

        <Self as ModularInput>::write_event_xml(&scheme).map_err(anyhow::Error::from)?;
        Ok(())
    }
}
