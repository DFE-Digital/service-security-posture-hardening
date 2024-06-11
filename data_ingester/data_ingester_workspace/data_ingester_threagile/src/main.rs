use anyhow::{Result, Context};
use serde::Serialize;
use serde_with::serde_as;
use std::fs::{self, File};
use std::io::prelude::*;
use std::collections::HashMap;
use serde_with::DisplayFromStr;
mod model;
mod risks;
use model::Model;
use data_ingester_splunk_search::acs::Acs;
use data_ingester_splunk_search::search_client::SplunkApiClient;
use serde::Deserialize;
use tracing::{debug, info, instrument, subscriber::DefaultGuard, warn};
use tracing_subscriber::{layer::SubscriberExt, EnvFilter, Registry};
use std::process::Command;
use std::os::unix::fs::PermissionsExt;

#[tokio::main]
async fn main() -> Result<()> {
    let stdout_log = tracing_subscriber::fmt::layer()
        .with_ansi(false)
        .compact()
        .with_writer(std::io::stderr);
    let subscriber = Registry::default().with(stdout_log).with(
        EnvFilter::from_default_env()
            .add_directive("info".parse().context("Parsing default log level")?),
    );
    tracing::subscriber::set_default(subscriber);

    let key_vault_name =
        std::env::var("KEY_VAULT_NAME").context("Getting key vault name from env:KEY_VAULT_NAME")?;

    let secrets = data_ingester_supporting::keyvault::get_keyvault_secrets(&key_vault_name)
            .await
        .context("Getting KeyVault secrets")?;
    
    // TODO anything but this
    // let stack = secrets
    //     .splunk_host
    //     .as_ref()
    //     .context("Getting splunk_host secret")?
    //     .split('.')
    //     .next()
    //     .context("Get host url ")?
    //     .split('-')
    //     .map(|s| s.to_string())
    //     .last()
    //     .context("Get stack from url")?;


    let stack = secrets
        .splunk_host
        .as_ref()
        .context("Getting splunk_host secret")?
        .split(':')
        .next()
        .context("Get host url ")?
        .to_string();
    
    // info!("Building ACS");
    // let acs_token = secrets
    //     .splunk_acs_token
    //     .as_ref()
    //     .context("Getting splunk_acs_token secret")?;
    // let mut acs = Acs::new(&stack, acs_token).context("Building Acs Client")?;

    // let ip_allow_list = acs
    //     .list_search_api_ip_allow_list()
    //     .await
    //     .context("Getting IP allow list")?;
    // info!("Splunk IP Allow list before add: {:?}", ip_allow_list);

    // info!("Granting access for current IP");
    // acs.grant_access_for_current_ip()
    //     .await
    //     .context("Granting access for current IP")?;

    // let ip_allow_list = acs
    //     .list_search_api_ip_allow_list()
    //     .await
    //     .context("Getting IP allow list")?;
    // info!("Splunk IP Allow list after add: {:?}", ip_allow_list);

    let search_token = secrets
        .splunk_search_token
        .as_ref()
        .context("Getting splunk_search_token secret")?;
    //let url = format!("https://{}.splunkcloud.com:8089", &stack);
    let url = format!("https://{}:8089", &stack);    
    let search_client = SplunkApiClient::new(&url, search_token)
        .context("Creating Splunk search client")?
        .set_app("DCAP_DEV");

    info!("Running search");
    let search_results = search_client
        .run_search::<serde_json::Value>("| savedsearch ssphp_get_list_service_resources")
        .await
        .context("Running Splunk Search")?;

    dbg!(&search_results);

    let search_results = search_client
        .run_search::<model::SplunkResult>("| savedsearch ssphp_get_list_service_resources")
        .await
        .context("Running Splunk Search")?;

    dbg!(&search_results);
    
    let mut collection = HashMap::new();
    for result in search_results {
        collection.insert(result.resource_id.to_string(), result.into());
    }
    let ta = model::TechnicalAssets(collection);
    
    let mut model = model::Model::test_data();
    model.technical_assets = ta;
        
    model.write_file("/tmp/results_from_splunk.yaml");

    let current_exe = std::env::current_exe().context("Gettting current exe path")?;
    let current_exe_dir = current_exe.parent().context("No parent for current exe path")?;

    
    let threagile_bytes = include_bytes!("../threagile_bin/threagile");
    let threagile_path = current_exe_dir.join("threagile");
    let mut threagile_file = std::fs::File::create(&threagile_path).context("Unable to create 'threagile' bin")?;
    threagile_file.write_all(threagile_bytes).context("Unable to write 'threagile' bytes to file");
    let threagile_file_metadata = threagile_file.metadata().context("Unable to get 'threagile' metadata")?;
    dbg!(&threagile_file_metadata);
    let mut threagile_file_permissions = threagile_file_metadata.permissions();
    dbg!(&threagile_file_permissions);
    threagile_file_permissions.set_mode(0o100700);
    fs::set_permissions(&threagile_path, threagile_file_permissions);
    drop(threagile_file);
    
    let raa_calc_bytes = include_bytes!("../threagile_bin/raa_calc");
    let raa_calc_path = current_exe_dir.join("raa_calc");
    let mut raa_calc_file = std::fs::File::create(&raa_calc_path).context("Unable to create 'raa_calc' bin")?;
    raa_calc_file.write_all(raa_calc_bytes).context("Unable to write raa_calc bytes to file");
    let raa_calc_file_metadata = raa_calc_file.metadata().context("Unable to get raa_calc metadata")?;
    let mut raa_calc_file_permissions = raa_calc_file_metadata.permissions();
    raa_calc_file_permissions.set_mode(0o100700);
    fs::set_permissions(&raa_calc_path, raa_calc_file_permissions);
    drop(raa_calc_file);    
    

    let threagile_output = Command::new(&threagile_path).args(["analyze-model", "--model", "/tmp/results_from_splunk.yaml", "--verbose"]).output()?;

    println!("{}", String::from_utf8(threagile_output.stdout.clone())?);
                                      
    
    Ok(())
}


