#![feature(async_fn_in_trait)]

use anyhow::{anyhow, Context, Result};
use clap::Parser;
use modular_input::{Args, Input, ModularInput};
use std::io::{self};
use tracing::debug;
mod github;
use crate::azure::AzureMI;
use crate::github::GitHubMI;
mod azure;
mod azure_client;
mod symlinks;
use crate::symlinks::{current_exe_from_args, make_symlinks};

static SPLUNK_APP_NAME: &str = "TA_github_reader";

#[tokio::main]
async fn main() -> Result<()> {
    make_symlinks();
    tracing_subscriber::fmt()
        // Use a more compact, abbreviated log format
        .compact()
        // Display source code file paths
        .with_file(true)
        .with_level(true)
        .with_env_filter("github_reader=debug")
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

    let current_name = current_exe_from_args().context("Unable to get bin name")?;

    let args = Args::parse();
    debug!("current_name: {:?}", current_name);
    match current_name.as_str() {
        "github" => run::<GitHubMI>(&args).await?,
        "azure_client" => run::<AzureMI>(&args).await?,
        _ => return Err(anyhow!("Unknown binary name!")),
    };
    Ok(())
}

async fn run<T: ModularInput>(args: &Args) -> Result<()> {
    if args.scheme {
        <T>::scheme()?;
        return Ok(());
    }
    let input = Input::from_stdin()?;
    debug!("input: {:?}", &input);
    let actual_mi = <T>::from_input(&input).await?;
    if args.validate_arguments {
        actual_mi.validate_arguments().await?
    } else {
        actual_mi.run().await?
    }
    Ok(())
}
