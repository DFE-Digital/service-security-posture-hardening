use std::sync::Arc;

use crate::{acs::Acs, search_client::SplunkApiClient};
use anyhow::{Context, Result};
use data_ingester_supporting::keyvault::Secrets;
use tracing::info;

use data_ingester_splunk::splunk::Splunk;
pub async fn splunk_acs_test(secrets: Arc<Secrets>, _splunk: Arc<Splunk>) -> Result<()> {
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
    let search_token = secrets
        .splunk_search_token
        .as_ref()
        .context("Getting splunk_search_token secret")?;
    let search =
        SplunkApiClient::new(&stack, search_token).context("Creating Splunk search client")?;

    let ip_allow_list = acs
        .list_search_api_ip_allow_list()
        .await
        .context("Getting IP allow list")?;
    info!("Splunk IP Allow list after add: {:?}", ip_allow_list);

    info!("Running search");
    let results = search
        .run_search("| savedsearch ssphp_get_list_qualys_cve")
        .await
        .context("Running Splunk Search")?;
    info!(
        "Search results ... {}",
        &results.chars().take(200).collect::<String>()
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
