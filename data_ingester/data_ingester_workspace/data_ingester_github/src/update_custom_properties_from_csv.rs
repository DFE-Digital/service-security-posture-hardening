use anyhow::{Context, Result};
use clap::Parser;
use data_ingester_financial_business_partners::validator::Validator;
use data_ingester_github::custom_properties::{
    CustomProperty, Property, ServiceLineCleaner, SetOrgRepoCustomProperties,
};
use data_ingester_github::OctocrabGit;
use data_ingester_supporting::keyvault::{get_keyvault_secrets, Secrets};
use serde::Deserialize;
use std::collections::HashMap;
use std::env;
use std::sync::Arc;
use tracing::{debug, error, info};
use tracing_subscriber::EnvFilter;

#[derive(Parser, Debug)]
struct Args {
    #[arg(short, long)]
    csv: String,
}

static APP_NAME: &str = "github_csv_updater";

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();
    let args = Args::parse();

    let secrets = get_keyvault_secrets(
        &env::var("KEY_VAULT_NAME").expect("Need KEY_VAULT_NAME enviornment variable"),
    )
    .await
    .map(Arc::new)
    .expect("Must be able to get secrets from KEYVAULT");

    let custom_property_validator = Validator::from_splunk_fbp(secrets.clone())
        .await
        .map(Arc::new)
        .context("Building Custom Property Validator")?;

    let owner_data = load_csv(&args.csv)
        .await
        .context("loading & validating CSV")?;

    debug!(name="update_custom_properties_from_csv", operation="validator", custom_property_validtor=?custom_property_validator);

    owner_data
        .is_valid(&custom_property_validator)
        .context("CSV data is not valid")?;

    let github_clients = GitHubClients::from_secrets(secrets.clone())
        .await
        .context("Building GitHub Installation clients")?;

    let current_tags = current_tags(&github_clients, custom_property_validator.clone())
        .await
        .context("Getting current Custom Properties")?;

    debug!(name=APP_NAME, operation="current_tags", current_tags=?current_tags);

    for entry in owner_data.repos.iter() {
        let currently_valid_tags = current_tags
            .get(&entry.organization)
            .and_then(|repos| repos.get(&entry.repo_name))
            .and_then(|repo| repo.validation_errors.as_ref())
            .map(|validation| validation.valid)
            .unwrap_or(false);
        if currently_valid_tags {
            info!(name=APP_NAME, operation="Setting custom_properties", entry_from_csv=?entry, "skipping as current tags are valid");
            continue;
        }

        let client = github_clients
            .clients
            .get(&entry.organization)
            .with_context(|| {
                format!(
                    "Getting Github Installation client for {}",
                    entry.organization
                )
            })?;

        let setter = entry.into();
        info!(name=APP_NAME, operation="Setting custom properties", entry=?entry);

        let response = client
            .org_create_or_update_custom_property_value(&entry.organization, setter)
            .await
            .with_context(|| format!("Setting Custom properties for {:?}", entry))?;

        info!(name=APP_NAME, operation="Setting custom properties", response=?response);
    }

    Ok(())
}

async fn current_tags(
    github_clients: &GitHubClients,
    custom_property_validator: Arc<Validator>,
) -> Result<HashMap<String, HashMap<String, CustomProperty>>> {
    let mut current_tags: HashMap<String, HashMap<String, CustomProperty>> = HashMap::new();

    for (org_name, client) in github_clients.clients.iter() {
        let custom_properties = client
            .org_get_custom_property_values(org_name, Some(custom_property_validator.clone()))
            .await
            .with_context(|| format!("Getting Custom properties for : {}", org_name))?;

        custom_properties
            .custom_properties
            .into_iter()
            .for_each(|cp| {
                let _ = current_tags
                    .entry(org_name.to_string())
                    .and_modify(|org| {
                        let _ = org.insert(cp.repository_name.to_string(), cp);
                    })
                    .or_default();
            });
    }
    Ok(current_tags)
}

async fn load_csv(file_path: &str) -> Result<GitHubRepoCsv> {
    let owner_data = GitHubRepoCsv::from_file(file_path)
        .with_context(|| format!("Reading CSV file: {}", file_path))?;

    debug!(name="update_custom_properties_from_csv", operation="csv", csv_data=?owner_data);

    Ok(owner_data)
}

#[derive(Debug, Deserialize)]
struct GitHubRepoCsv {
    repos: Vec<GitHubRepoOwner>,
}

impl GitHubRepoCsv {
    fn from_file(csv_path: &str) -> Result<Self> {
        let mut rdr = csv::Reader::from_path(csv_path)
            .with_context(|| format!("Opening CSV file: {}", csv_path))?;
        let service_line_cleaner = ServiceLineCleaner::default();
        let records: Vec<GitHubRepoOwner> = rdr
            .deserialize::<GitHubRepoOwner>()
            .filter_map(|record| match record {
                Ok(mut record) => {
                    record.service_line = service_line_cleaner
                        .allowed_values_cleaner_to_github(&record.service_line)
                        .to_string();
                    Some(record)
                }
                Err(err) => {
                    error!(name="update_custom_properties_from_csv", opertaion="csv", error=?err);
                    None
                }
            })
            .collect();

        info!(
            name = APP_NAME,
            operation = "csv",
            records_len = records.len(),
            message = "Records read from csv"
        );

        Ok(GitHubRepoCsv { repos: records })
    }

    fn is_valid(&self, validator: &Validator) -> Result<()> {
        if self.repos.iter().map(|repo| {
            let result = validator.validate(Some(&repo.portfolio), Some(&repo.service_line), Some(&repo.product));
            if !result.valid {
                error!(name=APP_NAME, operation="vaildate CSV against FBP", csv_entry=?repo, validation_errors=?result);
            }
            result.valid
        }).all(|result| result) {
            Ok(())
        } else {
            anyhow::bail!("CSV validation errors");
        }
    }

    #[allow(dead_code)]
    fn as_set_org_repo_custom_properties(
        &self,
    ) -> HashMap<String, Vec<SetOrgRepoCustomProperties>> {
        let mut org_repo_custom_property_setters: HashMap<String, Vec<SetOrgRepoCustomProperties>> =
            HashMap::new();
        for repo in self.repos.iter() {
            let _ = org_repo_custom_property_setters
                .entry(repo.organization.clone())
                .and_modify(|vec| vec.push(repo.into()))
                .or_default();
        }
        org_repo_custom_property_setters
    }
}

#[derive(Debug, Deserialize)]
struct GitHubRepoOwner {
    organization: String,
    repo_name: String,
    portfolio: String,
    service_line: String,
    product: String,
}

impl From<&GitHubRepoOwner> for SetOrgRepoCustomProperties {
    fn from(value: &GitHubRepoOwner) -> Self {
        SetOrgRepoCustomProperties {
            repository_names: vec![value.repo_name.to_string()],
            properties: vec![
                Property::new_single_value("portfolio", &value.portfolio),
                Property::new_single_value("service_line", &value.service_line),
                Property::new_single_value("product", &value.product),
            ],
        }
    }
}

#[derive(Debug)]
struct GitHubClients {
    clients: HashMap<String, OctocrabGit>,
}

impl GitHubClients {
    async fn from_secrets(secrets: Arc<Secrets>) -> Result<GitHubClients> {
        let github_app = secrets.github_app.as_ref().context("No GitHub Secrets")?;
        let client = OctocrabGit::new_from_app(github_app).context("Build OctocrabGit")?;
        let installations = client
            .client
            .apps()
            .installations()
            .send()
            .await
            .context("Getting installations for github app")?;

        let mut clients = HashMap::new();

        for installation in installations {
            if installation.account.r#type != "Organization" {
                continue;
            }
            let installation_client = client
                .for_installation_id(installation.id)
                .await
                .context("build octocrabgit client")?;
            let org_name = installation.account.login.to_string();
            let _ = clients.insert(org_name, installation_client);
        }
        debug!(name=APP_NAME, operation="github", clients=?clients);
        Ok(GitHubClients { clients })
    }
}
