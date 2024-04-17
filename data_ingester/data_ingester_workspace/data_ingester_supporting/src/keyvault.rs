use anyhow::{Context, Result};
use azure_identity::DefaultAzureCredential;
use azure_security_keyvault::{KeyvaultClient, SecretClient};
use base64::prelude::*;
use std::sync::Arc;
use tokio::task::JoinHandle;
use tracing::{info, warn};

pub struct Secrets {
    pub splunk_host: Option<String>,
    pub splunk_token: Option<String>,
    pub splunk_acs_token: Option<String>,
    pub splunk_search_token: Option<String>,
    pub azure_client_id: Option<String>,
    pub azure_client_secret: Option<String>,
    pub azure_client_certificate: Option<String>,
    pub azure_client_organization: Option<String>,
    pub azure_tenant_id: Option<String>,
    pub aws_access_key_id: Option<String>,
    pub aws_secret_access_key: Option<String>,
    pub github_app: Option<GitHubApp>,
}

/// Store a Github App token
///
/// The key should be stored as base64 encoded DER format
///
/// openssl rsa -in private-key.pem -outform DER -traditional -out private-key.der
/// cat private-key.der | base64
pub struct GitHubApp {
    pub app_id: u64,
    pub private_key: Vec<u8>,
}

impl GitHubApp {
    /// Create a new Github App secret.
    /// 'private_key' should be a base64 encoded DER RSA key
    fn new(app_id: String, private_key: String) -> Result<Self> {
        Ok(Self {
            app_id: app_id.parse().context("Parse app ID as u64")?,
            private_key: BASE64_STANDARD
                .decode(private_key)
                .context("Base64 decode GitHub private key")?,
        })
    }
}

/// Spawn a future getting a secret to be await'd later
/// Speeds up secrets collection
fn get_secret(client: &SecretClient, name: &str) -> JoinHandle<Option<String>> {
    let client_ = (*client).clone();
    let name_ = name.to_string();
    tokio::spawn(async move {
        info!("KeyVault: getting '{}'", &name_);
        match client_.get(&name_).await {
            Ok(secret) => Some(secret.value.to_string()),
            Err(err) => {
                warn!("Keyvault: Error gettingt '{}': {:?}", &name_, err);
                None
            }
        }
    })
}

/// Get all the secrets from KeyVault
pub async fn get_keyvault_secrets(keyvault_name: &str) -> Result<Secrets> {
    info!("Getting Default Azure Credentials");
    let credential = Arc::new(DefaultAzureCredential::default());

    info!("KeyVault Secret Client created");
    let keyvault_url = format!("https://{keyvault_name}.vault.azure.net");
    let client = KeyvaultClient::new(&keyvault_url, credential.clone())
        .context("Creating key vault client")?
        .secret_client();

    let splunk_host = get_secret(&client, "splunk-host");
    let splunk_token = get_secret(&client, "splunk-token");
    let splunk_acs_token = get_secret(&client, "splunk-acs-token");
    let splunk_search_token = get_secret(&client, "splunk-search-token");
    let azure_client_id = get_secret(&client, "ad-client-id");
    let azure_client_secret = get_secret(&client, "ad-client-secret");
    // Secret is automatically created when generating a certificate in KeyVault
    let azure_client_certificate = get_secret(&client, "ad-client-certificate");
    let azure_client_organization = get_secret(&client, "ad-client-organization");
    let azure_tenant_id = get_secret(&client, "ad-tenant-id");
    let aws_access_key_id = get_secret(&client, "aws-access-key-id");
    let aws_secret_access_key = get_secret(&client, "aws-secret-access-key");
    let github_private_key_1 = get_secret(&client, "github-private-key-1");
    let github_app_id_1 = get_secret(&client, "github-app-id-1");

    let github_app = if let (Some(github_app_id_1), Some(github_private_key_1)) =
        (github_app_id_1.await?, github_private_key_1.await?)
    {
        Some(
            GitHubApp::new(github_app_id_1, github_private_key_1)
                .context("Building Github App Credentials")?,
        )
    } else {
        None
    };

    Ok(Secrets {
        splunk_host: splunk_host.await?,
        splunk_token: splunk_token.await?,
        splunk_acs_token: splunk_acs_token.await?,
        splunk_search_token: splunk_search_token.await?,
        azure_client_id: azure_client_id.await?,
        azure_client_secret: azure_client_secret.await?,
        azure_client_certificate: azure_client_certificate.await?,
        azure_client_organization: azure_client_organization.await?,
        azure_tenant_id: azure_tenant_id.await?,
        aws_access_key_id: aws_access_key_id.await?,
        aws_secret_access_key: aws_secret_access_key.await?,
        github_app,
    })
}
