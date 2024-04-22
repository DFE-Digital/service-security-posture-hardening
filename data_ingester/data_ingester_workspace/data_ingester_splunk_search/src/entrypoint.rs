use crate::{acs::Acs, search_client::SplunkApiClient};
use anyhow::{Context, Result};
use data_ingester_qualys::Qualys;
use data_ingester_splunk::splunk::{try_collect_send, Splunk};
use data_ingester_supporting::keyvault::Secrets;
use serde::Deserialize;
use std::sync::Arc;
use tracing::info;

/// Struct for results for the Splunk search Cve data
#[derive(Default, Debug, Clone, Deserialize)]
#[serde(transparent)]
struct Cve(String);

pub async fn splunk_acs_test(secrets: Arc<Secrets>, splunk: Arc<Splunk>) -> Result<()> {
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

    let ip_allow_list = acs
        .list_search_api_ip_allow_list()
        .await
        .context("Getting IP allow list")?;
    info!("Splunk IP Allow list before add: {:?}", ip_allow_list);

    info!("Granting access for current IP");
    acs.grant_access_for_current_ip()
        .await
        .context("Granting access for current IP")?;

    let ip_allow_list = acs
        .list_search_api_ip_allow_list()
        .await
        .context("Getting IP allow list")?;
    info!("Splunk IP Allow list after add: {:?}", ip_allow_list);

    let search_token = secrets
        .splunk_search_token
        .as_ref()
        .context("Getting splunk_search_token secret")?;
    let url = format!("https://{}.splunkcloud.com:8089", &stack);
    let search =
        SplunkApiClient::new(&url, search_token).context("Creating Splunk search client")?;

    info!("Running search");
    let search_results = search
        .run_search::<Cve>("| savedsearch ssphp_get_list_qualys_cve")
        .await
        .context("Running Splunk Search")?;

    info!(
        "Search results ... {:?}",
        &search_results.iter().take(2).collect::<Vec<&Cve>>()
    );

    info!("Removing current IP from Splunk Allow list");
    acs.remove_current_cidr()
        .await
        .context("Removing current IP from Splunk")?;

    let ip_allow_list = acs
        .list_search_api_ip_allow_list()
        .await
        .context("Getting IP allow list")?;
    info!("Splunk IP Allow list after remove: {:?}", ip_allow_list);

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
        .map(|cve| cve.0.to_owned())
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
