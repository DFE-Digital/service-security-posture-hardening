use anyhow::{Context, Result};
use data_ingester_splunk::splunk::{set_ssphp_run, Splunk, ToHecEvents};
use data_ingester_supporting::keyvault::Secrets;
use std::collections::HashMap;
use std::fs::{self};
use std::io::prelude::*;
use std::path::PathBuf;
use std::sync::Arc;
mod model;
mod risks;
use data_ingester_splunk_search::search_client::SplunkApiClient;
use std::os::unix::fs::PermissionsExt;
use std::process::Command;
use tracing::{debug, info};

fn extract_threagile() -> Result<PathBuf> {
    info!("Extracting threagile");

    let threagile_bytes = include_bytes!("../threagile_bin/threagile");
    let threagile_path = PathBuf::from("/tmp/threagile_bin");

    let mut threagile_file =
        std::fs::File::create(&threagile_path).context("Unable to create 'threagile' bin")?;
    threagile_file
        .write_all(threagile_bytes)
        .context("Unable to write 'threagile' bytes to file")?;
    let threagile_file_metadata = threagile_file
        .metadata()
        .context("Unable to get 'threagile' metadata")?;

    let mut threagile_file_permissions = threagile_file_metadata.permissions();

    threagile_file_permissions.set_mode(0o100700);
    fs::set_permissions(&threagile_path, threagile_file_permissions)?;
    drop(threagile_file);

    Ok(threagile_path)
}

pub async fn threagile(secrets: Arc<Secrets>, splunk: Arc<Splunk>) -> Result<()> {
    set_ssphp_run("threagile")?;
    info!("Extracting Threagile bins");
    let threagile_path = extract_threagile()?;

    let splunk_cloud_stack = secrets
        .splunk_cloud_stack
        .as_ref()
        .map(|stack| stack.as_str());

    let splunk_acs_token = secrets
        .splunk_acs_token
        .as_ref()
        .map(|token| token.as_str());

    let splunk_search_token = secrets
        .splunk_search_token
        .as_ref()
        .context("Getting splunk_search_token secret")?;

    let splunk_search_url = secrets
        .splunk_search_url
        .as_ref()
        .context("Getting splunk_search_url secret")?;

    let mut search_client = SplunkApiClient::new(
        &splunk_search_url,
        splunk_search_token,
        splunk_cloud_stack,
        splunk_acs_token,
    )
    .context("Creating Splunk search client")?
    .set_app("DCAP");

    search_client
        .open_acs()
        .await
        .context("Opening Splunk access via ACS")?;

    let search = "| savedsearch ssphp_get_list_service_resources";

    info!("Running splunk search '{}'", search);
    let search_results = search_client
        .run_search::<model::SplunkResult>(search)
        .await
        .context("Running Splunk Search")?;

    debug!("Splunk search results: {:?}", search_results);

    search_client
        .close_acs()
        .await
        .context("Closing Splunk access via ACS")?;

    let mut services: HashMap<String, Vec<model::SplunkResult>> = HashMap::new();

    for result in search_results {
        let service = services.entry(result.service_id.to_string()).or_default();
        service.push(result.clone());
    }

    info!("Found {} services", services.len());

    for (service, risks) in services {
        let mut collection = HashMap::new();
        for result in risks {
            let _ = collection.insert(result.resource_id.to_string(), result.into());
        }
        let ta = model::TechnicalAssets(collection);

        let mut model = model::Model::default();
        model.technical_assets = ta;

        let risks_path = format!("/tmp/{}_results_from_splunk.yaml", &service);
        info!("Writing risks file: {}", risks_path);
        model
            .write_file(&risks_path)
            .context("Writing risks file")?;
        info!("Running Threagile: {:?}", &threagile_path);

        let threagile_output = Command::new(&threagile_path)
            .args([
                "analyze-model",
                "--model",
                &risks_path,
                "--verbose",
                "--output",
                "/tmp",
                "--temp-dir",
                "/tmp",
            ])
            .output()?;

        info!(
            "Threagile status: {:?}\nstdout: {}\nstderr: {}",
            threagile_output.status,
            String::from_utf8(threagile_output.stdout.clone())?,
            String::from_utf8(threagile_output.stderr.clone())?,
        );

        info!("Reading risks.json");
        let risks = risks::RisksJson::from_file("/tmp/risks.json", &service)?;

        info!("Found {} risks for {}", risks.risks.len(), &service);

        let splunk_events = (&risks).to_hec_events()?;

        splunk.send_batch(&splunk_events).await?;
    }

    Ok(())
}

#[cfg(test)]
mod test {
    use std::sync::Arc;

    use anyhow::{Context, Result};
    use data_ingester_splunk::splunk::Splunk;
    use tracing_subscriber::{layer::SubscriberExt, EnvFilter, Registry};

    #[tokio::test]
    async fn test_threagile() -> Result<()> {
        let stdout_log = tracing_subscriber::fmt::layer()
            .with_ansi(false)
            .compact()
            .with_writer(std::io::stderr);
        let subscriber = Registry::default().with(stdout_log).with(
            EnvFilter::from_default_env()
                .add_directive("info".parse().context("Parsing default log level")?),
        );
        let _tracing_guard = tracing::subscriber::set_default(subscriber);

        let key_vault_name = std::env::var("KEY_VAULT_NAME")
            .context("Getting key vault name from env:KEY_VAULT_NAME")?;

        let secrets = data_ingester_supporting::keyvault::get_keyvault_secrets(&key_vault_name)
            .await
            .context("Getting KeyVault secrets")?;

        let splunk = Splunk::new(
            &secrets.splunk_host.as_ref().context("No value")?,
            &secrets.splunk_token.as_ref().context("No value")?,
        )?;

        super::threagile(Arc::new(secrets), Arc::new(splunk)).await?;
        Ok(())
    }
}
