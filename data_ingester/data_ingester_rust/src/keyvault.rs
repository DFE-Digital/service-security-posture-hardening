use std::{error::Error, sync::Arc};

use anyhow::{Context, Result};
use azure_identity::DefaultAzureCredential;
use azure_security_keyvault::KeyvaultClient;

pub(crate) struct Secrets {
    pub(crate) splunk_host: String,
    pub(crate) splunk_token: String,
    pub(crate) azure_client_id: String,
    pub(crate) azure_client_secret: String,
    pub(crate) azure_tenant_id: String,
}

pub async fn get_keyvault_secrets(keyvault_name: &str) -> Result<Secrets> {
    let keyvault_url = format!("https://{keyvault_name}.vault.azure.net");
    let credential = Arc::new(DefaultAzureCredential::default());
    let client = KeyvaultClient::new(&keyvault_url, credential)?.secret_client();

    Ok(Secrets {
        splunk_host: client.get("splunk-host").await?.value,
        splunk_token: client.get("splunk-token").await?.value,
        azure_client_id: client.get("ad-client-id").await?.value,
        azure_client_secret: client.get("ad-client-secret").await?.value,
        azure_tenant_id: client.get("ad-tenant-id").await?.value,
    })
}
