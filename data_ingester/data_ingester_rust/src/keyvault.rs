use std::sync::Arc;

use anyhow::Result;
use azure_identity::DefaultAzureCredential;
use azure_security_keyvault::KeyvaultClient;

pub(crate) struct Secrets {
    pub(crate) splunk_host: String,
    pub(crate) splunk_token: String,
    pub(crate) azure_client_id: String,
    pub(crate) azure_client_secret: String,
    pub(crate) azure_client_certificate: String,
    pub(crate) azure_client_organization: String,
    pub(crate) azure_tenant_id: String,
}

pub async fn get_keyvault_secrets(keyvault_name: &str) -> Result<Secrets> {
    let keyvault_url = format!("https://{keyvault_name}.vault.azure.net");
    let credential = Arc::new(DefaultAzureCredential::default());
    let secret_client = KeyvaultClient::new(&keyvault_url, credential.clone())?.secret_client();
    let certificate_client = KeyvaultClient::new(&keyvault_url, credential)?.certificate_client();

    Ok(Secrets {
        splunk_host: secret_client.get("splunk-host").await?.value,
        splunk_token: secret_client.get("splunk-token").await?.value,
        azure_client_id: secret_client.get("ad-client-id").await?.value,
        azure_client_secret: secret_client.get("ad-client-secret").await?.value,
        // Secret is automatically created when generating a certificate in KeyVault
        azure_client_certificate: secret_client.get("ad-client-certificate").await?.value,
        azure_client_organization: secret_client.get("ad-client-organization").await?.value,
        azure_tenant_id: secret_client.get("ad-tenant-id").await?.value,
    })
}
