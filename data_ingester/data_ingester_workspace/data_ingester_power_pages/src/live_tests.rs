use std::env;

use anyhow::{Context, Result};
use data_ingester_splunk::splunk::{set_ssphp_run, Splunk, SplunkTrait};
use data_ingester_supporting::keyvault::get_keyvault_secrets;

use crate::entrypoint;
use crate::SSPHP_RUN_KEY;

#[tokio::test]
async fn live_power_pages_entrypoint() -> Result<()> {
    let _ = set_ssphp_run(SSPHP_RUN_KEY);
    let secrets = get_keyvault_secrets(
        &env::var("KEY_VAULT_NAME").expect("Need KEY_VAULT_NAME environment variable"),
    )
    .await?;
    let splunk = Splunk::new(
        secrets.splunk_host.as_ref().context("No splunk_host")?,
        secrets.splunk_token.as_ref().context("No splunk_token")?,
        true,
    )?;
    entrypoint(std::sync::Arc::new(secrets), std::sync::Arc::new(splunk)).await
}
