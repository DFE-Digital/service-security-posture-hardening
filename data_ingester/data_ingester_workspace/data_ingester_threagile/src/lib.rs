use anyhow::{Context, Result};
use data_ingester_splunk::splunk::{Splunk, ToHecEvents};
use data_ingester_supporting::keyvault::Secrets;
use serde_json::Value;
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{prelude::*, BufReader};
use std::path::PathBuf;
use std::sync::Arc;
mod model;
mod risks;
use data_ingester_splunk_search::acs::Acs;
use data_ingester_splunk_search::search_client::SplunkApiClient;
use std::os::unix::fs::PermissionsExt;
use std::process::Command;
use tracing::info;

fn extract_threagile() -> Result<PathBuf> {
    info!("Extracting threagile");
    let current_exe = std::env::current_exe().context("Gettting current exe path")?;
    dbg!(&current_exe);
    let current_exe_dir = current_exe
        .parent()
        .context("No parent for current exe path")?;
    dbg!(&current_exe_dir);
    let threagile_bytes = include_bytes!("../threagile_bin/threagile");
    let threagile_path = current_exe_dir.join("threagile_bin");
    let threagile_path = PathBuf::from("/tmp/threagile_bin");
    dbg!(&threagile_path);
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


    // info!("Extracting raa_calc");
    // let raa_calc_bytes = include_bytes!("../threagile_bin/raa_calc");
    // let raa_calc_path = current_exe_dir.join("raa_calc");
    // let raa_calc_path = PathBuf::from("/tmp/raa_calc");
    // let mut raa_calc_file =
    //     std::fs::File::create(&raa_calc_path).context("Unable to create 'raa_calc' bin")?;
    // raa_calc_file
    //     .write_all(raa_calc_bytes)
    //     .context("Unable to write raa_calc bytes to file")?;
    // let raa_calc_file_metadata = raa_calc_file
    //     .metadata()
    //     .context("Unable to get raa_calc metadata")?;
    // let mut raa_calc_file_permissions = raa_calc_file_metadata.permissions();
    // raa_calc_file_permissions.set_mode(0o100700);
    // fs::set_permissions(&raa_calc_path, raa_calc_file_permissions)?;
    // drop(raa_calc_file);
    Ok(threagile_path)
}

pub async fn threagile(secrets: Arc<Secrets>, splunk: Arc<Splunk>) -> Result<()> {
    info!("Extracting Threagile bins");
    let threagile_path = extract_threagile()?;

    // TODO anything but this
    let stack = secrets
        .splunk_host
        .as_ref()
        .context("Getting splunk_host secret")?
        .split('.')
        .next()
        .context("Get host url ")?
        .split('-')
        .map(|s| s.to_string())
        .last()
        .context("Get stack from url")?;

    info!("Building ACS");
    let acs_token = secrets
        .splunk_acs_token
        .as_ref()
        .context("Getting splunk_acs_token secret")?;

    let mut acs = Acs::new(&stack, acs_token).context("Building Acs Client")?;

    info!("Granting access for current IP");
    acs.grant_access_for_current_ip()
        .await
        .context("Granting access for current IP")?;

    let search_token = secrets
        .splunk_search_token
        .as_ref()
        .context("Getting splunk_search_token secret")?;

    // TODO Add this as a secret
    let url = format!("https://{}.splunkcloud.com:8089", &stack);

    // let url = format!("https://{}:8089", &stack);
    let search_client = SplunkApiClient::new(&url, search_token)
        .context("Creating Splunk search client")?
        .set_app("DCAP_DEV");

    info!("Running splunk search ssphp_get_list_service_resources_DEV");
    let search_results = search_client
        // TODO Remove '_DEV' before merge
        .run_search::<model::SplunkResult>("| savedsearch ssphp_get_list_service_resources_DEV")
        .await
        .context("Running Splunk Search")?;

    info!("Splunk search results: {:?}", search_results);

    let mut services: HashMap<String, Vec<model::SplunkResult>> = HashMap::new();

    for result in search_results {
        let service = services.entry(result.service_id.to_string()).or_default();
        service.push(result.clone());
    }

    info!("Found {} services", services.len());

    for (service, risks) in services {
        let mut collection = HashMap::new();
        for result in risks {
            collection.insert(result.resource_id.to_string(), result.into());
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
        tracing::subscriber::set_default(subscriber);

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
