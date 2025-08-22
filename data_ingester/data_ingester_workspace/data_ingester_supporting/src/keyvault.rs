use crate::dev_ops_pats::{azure_dev_ops_pats, AdoDevOpsPat};
use anyhow::{Context, Result};
use azure_identity::DefaultAzureCredential;
use azure_identity::TokenCredentialOptions;
use azure_security_keyvault::prelude::KeyVaultGetSecretResponse;
use azure_security_keyvault::prelude::KeyVaultGetSecretsResponse;
use azure_security_keyvault::{KeyvaultClient, SecretClient};
use base64::prelude::*;
use futures::StreamExt;
use serde::Serialize;
use std::sync::Arc;
use tokio::task::JoinHandle;
use tracing::{info, warn};

pub struct Secrets {
    pub splunk_host: Option<String>,
    pub splunk_token: Option<String>,
    pub ian_splunk_host: Option<String>,
    pub ian_splunk_token: Option<String>,
    pub splunk_acs_token: Option<String>,
    pub splunk_search_token: Option<String>,
    pub splunk_search_url: Option<String>,
    pub splunk_cloud_stack: Option<String>,
    pub azure_client_id: Option<String>,
    pub azure_client_secret: Option<String>,
    pub azure_client_certificate: Option<String>,
    pub azure_client_organization: Option<String>,
    pub azure_tenant_id: Option<String>,
    pub aws_access_key_id: Option<String>,
    pub aws_secret_access_key: Option<String>,
    pub github_app: Option<GitHubApp>,
    pub qualys_username: Option<String>,
    pub qualys_password: Option<String>,
    pub sonar_api_key: Option<String>,
    pub sonar_orgs: Option<Vec<String>>,
    pub mssql_host: Option<String>,
    pub mssql_port: Option<String>,
    pub mssql_db: Option<String>,
    pub mssql_username: Option<String>,
    pub mssql_password: Option<String>,
    pub ado_pats: Vec<AdoDevOpsPat>,
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
                warn!("Keyvault: Error getting '{}': {:?}", &name_, err);
                None
            }
        }
    })
}

/// Get all the secrets from KeyVault
pub async fn get_keyvault_secrets(keyvault_name: &str) -> Result<Secrets> {
    info!("Getting Default Azure Credentials");
    let credential = Arc::new(
        DefaultAzureCredential::create(TokenCredentialOptions::default())
            .context("Unable to build default Azure Credentials")?,
    );

    info!("KeyVault Secret Client created");
    let keyvault_url = format!("https://{keyvault_name}.vault.azure.net");
    let client = KeyvaultClient::new(&keyvault_url, credential.clone())
        .context("Creating key vault client")?
        .secret_client();

    let splunk_host = get_secret(&client, "splunk-host");
    let splunk_token = get_secret(&client, "splunk-token");
    let ian_splunk_host = get_secret(&client, "ian-splunk-host");
    let ian_splunk_token = get_secret(&client, "ian-splunk-token");
    let splunk_acs_token = get_secret(&client, "splunk-acs-token");
    let splunk_search_token = get_secret(&client, "splunk-search-token");
    let splunk_search_url = get_secret(&client, "splunk-search-url");
    let splunk_cloud_stack = get_secret(&client, "splunk-cloud-stack");
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
    let qualys_username = get_secret(&client, "qualys-username");
    let qualys_password = get_secret(&client, "qualys-password");
    let sonar_api_key = get_secret(&client, "sonar-api-key");
    let sonar_orgs = get_secret(&client, "sonar-orgs");
    let mssql_host = get_secret(&client, "mssql-host");
    let mssql_db = get_secret(&client, "mssql-db");
    let mssql_port = get_secret(&client, "mssql-port");
    let mssql_username = get_secret(&client, "mssql-username");
    let mssql_password = get_secret(&client, "mssql-password");

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

    let secrets = client
        .list_secrets()
        .into_stream()
        .filter_map(|result| async move {
            match result {
                Ok(result) => Some(result.value),
                Err(_) => None,
            }
        })
        .concat()
        .await;

    let ado_pats = azure_dev_ops_pats(&client, &secrets).await;

    Ok(Secrets {
        ian_splunk_host: ian_splunk_host.await?,
        ian_splunk_token: ian_splunk_token.await?,
        splunk_host: splunk_host.await?,
        splunk_token: splunk_token.await?,
        splunk_acs_token: splunk_acs_token.await?,
        splunk_search_token: splunk_search_token.await?,
        splunk_search_url: splunk_search_url.await?,
        splunk_cloud_stack: splunk_cloud_stack.await?,
        azure_client_id: azure_client_id.await?,
        azure_client_secret: azure_client_secret.await?,
        azure_client_certificate: azure_client_certificate.await?,
        azure_client_organization: azure_client_organization.await?,
        azure_tenant_id: azure_tenant_id.await?,
        aws_access_key_id: aws_access_key_id.await?,
        aws_secret_access_key: aws_secret_access_key.await?,
        github_app,
        qualys_username: qualys_username.await?,
        qualys_password: qualys_password.await?,
        sonar_api_key: sonar_api_key.await?,
        sonar_orgs: sonar_orgs
            .await?
            .map(|s| s.split(",").map(|s| s.to_string()).collect()),
        mssql_host: mssql_host.await?,
        mssql_port: mssql_port.await?,
        mssql_db: mssql_db.await?,
        mssql_username: mssql_username.await?,
        mssql_password: mssql_password.await?,
        ado_pats,
    })
}

/// Get all secret name & attributes, but not the secret value. Used
/// for healthcheck reporting on expired secrets.
pub async fn secret_health_check(keyvault_name: &str) -> Result<Vec<SecretAttributes>> {
    let credential = Arc::new(
        DefaultAzureCredential::create(TokenCredentialOptions::default())
            .context("Unable to build default Azure Credentials")?,
    );

    info!("KeyVault Secret Client created");
    let keyvault_url = format!("https://{keyvault_name}.vault.azure.net");
    let client = KeyvaultClient::new(&keyvault_url, credential.clone())
        .context("Creating key vault client")?
        .secret_client();

    let list_secret_response = client
        .list_secrets()
        .into_stream()
        .filter_map(|result| async move {
            match result {
                Ok(result) => Some(result.value),
                Err(_) => None,
            }
        })
        .concat()
        .await;

    let mut attributes = vec![];

    for secret in list_secret_response {
        let secret_attributes: SecretAttributes = client
            .get(secret.id)
            .await
            .map(|response| (keyvault_name.to_owned(), response).into())?;
        attributes.push(secret_attributes);
    }

    Ok(attributes)
}

#[derive(Debug, Serialize)]
pub struct SecretAttributes {
    pub id: String,
    pub keyvault_id: String,
    enabled: bool,
    expires_on: Option<i64>,
    created_on: i64,
    updated_on: i64,
    recovery_level: String,
}

impl From<(String, KeyVaultGetSecretResponse)> for SecretAttributes {
    fn from((keyvault_id, value): (String, KeyVaultGetSecretResponse)) -> Self {
        SecretAttributes {
            id: value.id,
            keyvault_id,
            enabled: value.attributes.enabled,
            expires_on: value.attributes.expires_on.map(|t| t.unix_timestamp()),
            created_on: value.attributes.created_on.unix_timestamp(),
            updated_on: value.attributes.updated_on.unix_timestamp(),
            recovery_level: value.attributes.recovery_level,
        }
    }
}
