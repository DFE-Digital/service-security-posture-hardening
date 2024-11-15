use anyhow::Result;
use clap::Parser;
use data_ingester_github::OctocrabGit;
use std::collections::HashMap;

#[derive(Parser, Debug)]
struct Args {
    #[arg(short, long)]
    csv: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    Ok(())
}

struct GitHubRepoCsv {
    repos: Vec<GitHubRepoOwner>,
}

struct GitHubRepoOwner {
    organization: OrgName,
    repo_name: String,
    portfolio: String,
    product: String,
}

struct OrgName(String);

struct GitHubClients {
    clients: HashMap<String, OctocrabGit>,
}
