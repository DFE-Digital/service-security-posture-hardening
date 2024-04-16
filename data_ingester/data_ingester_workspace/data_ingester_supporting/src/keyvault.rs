use std::sync::Arc;

use anyhow::{Context, Result};

use azure_identity::DefaultAzureCredential;
use azure_security_keyvault::KeyvaultClient;
use base64::prelude::*;
use futures::join;

pub struct Secrets {
    pub splunk_host: String,
    pub splunk_token: String,
    pub splunk_acs_token: String,
    pub azure_client_id: String,
    pub azure_client_secret: String,
    pub azure_client_certificate: String,
    pub azure_client_organization: String,
    pub azure_tenant_id: String,
    pub aws_access_key_id: String,
    pub aws_secret_access_key: String,
    pub github: GitHub,
}

/// openssl rsa -in private-key.pem -outform DER -traditional -out private-key.der

pub struct GitHubApp {
    pub app_id: u64,
    pub private_key: Vec<u8>,
}

pub struct GitHubPat {
    pub pat: String,
    pub orgs: Vec<String>,
}

impl GitHubPat {
    fn new(github_pat: String, github_orgs: String) -> Self {
        Self {
            pat: github_pat,
            orgs: github_orgs.split(';').map(String::from).collect(),
        }
    }
}

pub struct GitHub {
    pub app: Option<GitHubApp>,
    pub pat: Option<GitHubPat>,
}

impl GitHubApp {
    fn new(app_id: String, private_key: String) -> Result<Self> {
        Ok(Self {
            app_id: app_id.parse().context("Parse app ID as u64")?,
            private_key: BASE64_STANDARD
                .decode(private_key)
                .context("Base64 decode GitHub private key")?,
        })
    }
}

pub async fn get_keyvault_secrets(keyvault_name: &str) -> Result<Secrets> {
    let keyvault_url = format!("https://{keyvault_name}.vault.azure.net");
    let credential = Arc::new(DefaultAzureCredential::default());
    eprintln!("KeyVault Secret Client created");
    let secret_client = KeyvaultClient::new(&keyvault_url, credential.clone()).context("Creating key vault client")?.secret_client();

    let splunk_host = secret_client.get("splunk-host").into_future();
    let splunk_token = secret_client.get("splunk-token").into_future();
    let splunk_acs_token = secret_client.get("splunk-acs-token").into_future();    
    let azure_client_id = secret_client.get("ad-client-id").into_future();
    let azure_client_secret = secret_client.get("ad-client-secret").into_future();
    // Secret is automatically created when generating a certificate in KeyVault
    let azure_client_certificate = secret_client.get("ad-client-certificate").into_future();
    let azure_client_organization = secret_client.get("ad-client-organization").into_future();
    let azure_tenant_id = secret_client.get("ad-tenant-id").into_future();
    let aws_access_key_id = secret_client.get("aws-access-key-id").into_future();
    let aws_secret_access_key = secret_client.get("aws-secret-access-key").into_future();
    let github_private_key_1 = secret_client.get("github-private-key-1").into_future();
    let github_app_id_1 = secret_client.get("github-app-id-1").into_future();
    let github_pat = secret_client.get("github-pat").into_future();
    let github_pat_orgs = secret_client.get("github-pat-orgs").into_future();

    let (
        splunk_host,
        splunk_token,
        splunk_acs_token,
        azure_client_id,
        azure_client_secret,
        azure_client_certificate,
        azure_client_organization,
        azure_tenant_id,
        aws_access_key_id,
        aws_secret_access_key,
        github_private_key_1,
        github_app_id_1,
        github_pat,
        github_pat_orgs,
    ) = join!(
        splunk_host,
        splunk_token,
        splunk_acs_token,
        azure_client_id,
        azure_client_secret,
        azure_client_certificate,
        azure_client_organization,
        azure_tenant_id,
        aws_access_key_id,
        aws_secret_access_key,
        github_private_key_1,
        github_app_id_1,
        github_pat,
        github_pat_orgs
    );

    let github_app = if let (Ok(github_app_id_1), Ok(github_private_key_1)) =
        (github_app_id_1, github_private_key_1)
    {
        Some(
            GitHubApp::new(github_app_id_1.value, github_private_key_1.value)
                .context("Building Github App Credentials")?,
        )
    } else {
        None
    };

    let github_pat = if let (Ok(github_pat), Ok(github_pat_orgs)) = (github_pat, github_pat_orgs) {
        Some(GitHubPat::new(github_pat.value, github_pat_orgs.value))
    } else {
        None
    };

    // TODO change secret types to Option<T> and only pass correct secrets to functions.
    Ok(Secrets {
        splunk_host: splunk_host.map(|s| s.value).unwrap_or_default(),
        splunk_token: splunk_token.map(|s| s.value).unwrap_or_default(),
        splunk_acs_token: splunk_acs_token.map(|s| s.value).unwrap_or_default(),
        azure_client_id: azure_client_id.map(|s| s.value).unwrap_or_default(),
        azure_client_secret: azure_client_secret.map(|s| s.value).unwrap_or_default(),
        azure_client_certificate: azure_client_certificate
            .map(|s| s.value)
            .unwrap_or_default(),
        azure_client_organization: azure_client_organization
            .map(|s| s.value)
            .unwrap_or_default(),
        azure_tenant_id: azure_tenant_id.map(|s| s.value).unwrap_or_default(),
        aws_access_key_id: aws_access_key_id.map(|s| s.value).unwrap_or_default(),
        aws_secret_access_key: aws_secret_access_key.map(|s| s.value).unwrap_or_default(),
        github: GitHub {
            app: github_app,
            pat: github_pat,
        },
    })
}
