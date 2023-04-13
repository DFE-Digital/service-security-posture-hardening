#![feature(async_fn_in_trait)]
use anyhow::Result;
use modular_input::Event;
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
                let new_event = cloned_event.data_from(&repo)?;
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
                let new_event = cloned_event.data_from(&member)?;
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
        let new_event = template_event.clone().data_from(&rate_limit)?;
        event_writer.send(new_event)?;
        Ok(())
    }

    //async fn
}
