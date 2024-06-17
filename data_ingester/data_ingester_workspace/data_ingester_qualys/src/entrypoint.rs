use crate::Qualys;
use anyhow::{Context, Result};
use data_ingester_splunk::splunk::{set_ssphp_run, try_collect_send, Splunk};
use data_ingester_splunk_search::{acs::Acs, search_client::SplunkApiClient};
use data_ingester_supporting::keyvault::Secrets;
use serde::Deserialize;
use std::sync::Arc;
use tracing::{debug, info};

/// Struct for results for the Splunk search Cve data
#[derive(Default, Debug, Clone, Deserialize)]
struct Cve {
    cve: String,
}

pub async fn qualys_qvs(secrets: Arc<Secrets>, splunk: Arc<Splunk>) -> Result<()> {
    set_ssphp_run("qualys")?;

    let splunk_cloud_stack = secrets.splunk_cloud_stack.as_ref().map(|stack| stack.as_str());

    let splunk_acs_token = secrets.splunk_acs_token.as_ref().map(|token| token.as_str());

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

    let search = "| savedsearch  ssphp_get_list_qualys_cve";

    info!("Running splunk search '{}'", search);
    let search_results = search_client
        .run_search::<Cve>(search)
        .await
        .context("Running Splunk Search")?;

    search_client
        .close_acs()
        .await
        .context("Closing Splunk access via ACS")?;

    debug!("Splunk search results: {:?}", search_results);

    info!(
        "Search results ... {:?}",
        &search_results.iter().take(2).collect::<Vec<&Cve>>()
    );

    let mut qualys_client = Qualys::new(
        secrets
            .qualys_username
            .as_ref()
            .context("No qualys Username")?,
        secrets
            .qualys_password
            .as_ref()
            .context("No Qualys password")?,
    )?;

    info!("Getting data from Qualys QVS");
    let cves = search_results
        .iter()
        .map(|cve| cve.cve.to_owned())
        .collect::<Vec<String>>();
    let qualys_command = qualys_client.get_qvs(&cves);

    try_collect_send("Qualys vulnerability score", qualys_command, &splunk).await?;

    info!("Done");
    Ok(())
}

#[test]
fn test_foo() {
    let stack = "http-inputs-dfe.splunkcloud.com"
        .split('.')
        .next()
        .context("Get host url ")
        .unwrap()
        .split('-')
        .map(|s| s.to_string())
        .last()
        .context("Get stack from url")
        .unwrap();
    dbg!(stack);
    assert!(false);
}
