#![feature(async_fn_in_trait)]

use anyhow::{Context, Result};
use clap::Parser;
use futures::join;
use modular_input::{Args, Event, Input, ModularInput, Scheme};
use octorust::types::MinimalRepository;
use std::io::{self};
use std::time::SystemTime;
use tracing::instrument;
use tracing::{debug, info};
mod github;
use crate::github::GitHub;
mod azure;
// use crate::azure::AzureClient;
#[tokio::main]
#[instrument]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        // Use a more compact, abbreviated log format
        .compact()
        // Display source code file paths
        .with_file(true)
        .with_level(true)
        .with_env_filter("app=debug")
        // Display source code line numbers
        .with_line_number(true)
        // Display the thread ID an event was recorded on
        .with_thread_ids(true)
        // Don't display the event's target (module path)
        .with_target(false)
        .with_writer(io::stderr)
        .compact()
        .with_ansi(false)
        // Build the subscriber
        .try_init()
        .unwrap();

    let args = Args::parse();

    if args.scheme {
        GitHubMI::default().scheme()?;
        std::process::exit(0);
    }

    let input = Input::from_stdin()?;
    debug!("{:?}", &input);

    let org = input.param_by_name("org").context("No Organisation")?;

    let github_token = input
        .param_by_name("github_token")
        .context("No github Token")?;

    let mi = GitHubMI {
        org: org.to_string(),
        github_token: github_token.to_string(),
        repos: vec![],
    };

    match (args.scheme, args.validate_arguments) {
        (true, false) => {
            info!("scheme");
            mi.scheme()?;
            std::process::exit(0);
        }
        (false, true) => {
            info!("validate arguments");
            mi.validate_arguments(&input)
                .await
                .context("Failed to validate arguments")?;
            std::process::exit(0);
        }
        (false, false) => {
            info!("run");
            mi.run().await?;
        }
        (true, true) => {
            info!("incorrect options");
            std::process::exit(1);
        }
    }

    Ok(())
}

#[derive(Debug, Default)]
pub struct GitHubMI {
    org: String,
    github_token: String,
    repos: Vec<MinimalRepository>,
}

// impl<'a> ModularInput for Foo<'a> {
impl ModularInput for GitHubMI {
    //#[instrument]
    async fn run(&self) -> Result<()> {
        let time = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)?
            .as_secs_f64();

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

    async fn validate_arguments(&self, input: &Input) -> Result<()> {
        let org = input.param_by_name("org").context("No `org` parameter!")?;
        let github_token = input
            .param_by_name("token")
            .context("No `token` parameter!")?;
        GitHub::new(github_token)
            .await
            .check_access(org)
            .await
            .context(format!(
                "Unable to access Org:{} with token:{}...",
                org,
                &github_token.chars().take(8).collect::<String>()
            ))?;
        info!("Arguments valid!");
        Ok(())
    }

    #[instrument]
    fn scheme(&self) -> Result<()> {
        let scheme = Scheme {
            title: "github".to_string(),
            description: "test description".to_string(),
            streaming_mode: "xml".to_string(),
            use_single_instance: false,
        };

        self.write_event_xml(&scheme).map_err(anyhow::Error::from)?;
        Ok(())
    }
}
