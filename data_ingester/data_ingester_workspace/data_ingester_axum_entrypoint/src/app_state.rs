use anyhow::Context;
use anyhow::Result;
use data_ingester_splunk::splunk::Splunk;
use data_ingester_supporting::keyvault::get_keyvault_secrets;
use data_ingester_supporting::keyvault::Secrets;
use std::env;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::info;

/// AppState for all requests
#[derive(Clone)]
pub(crate) struct AppState {
    /// Splunk Client
    pub(crate) splunk: Arc<Splunk>,
    /// Secret
    pub(crate) secrets: Arc<Secrets>,

    /// Lock for AWS to stop concurrent executions
    pub(crate) aws_lock: Arc<Mutex<()>>,

    /// Lock for Azure to stop concurrent executions
    pub(crate) azure_lock: Arc<Mutex<()>>,

    /// Lock for azure_resource_graph to stop concurrent executions        
    pub(crate) azure_resource_graph_lock: Arc<Mutex<()>>,

    /// Lock for github to stop concurrent executions
    pub(crate) github_lock: Arc<Mutex<()>>,

    /// Lock for m365 to stop concurrent executions            
    pub(crate) m365_lock: Arc<Mutex<()>>,

    /// Lock for powershell to stop concurrent executions        
    pub(crate) powershell_lock: Arc<Mutex<()>>,
    /// Is powershell installed in our function?
    pub(crate) powershell_installed: Arc<Mutex<bool>>,

    /// Lock for splunk_test to stop concurrent executions            
    pub(crate) splunk_test_lock: Arc<Mutex<()>>,

    /// Lock for threagile to stop concurrent executions            
    pub(crate) threagile_lock: Arc<Mutex<()>>,
}

impl AppState {
    /// Create a new AppState
    pub(crate) async fn new() -> Result<Self> {
        let secrets = AppState::get_secrets().await?;
        let splunk = AppState::create_splunk_client(&secrets)?;
        Ok(Self {
            secrets: Arc::new(secrets),
            splunk: Arc::new(splunk),

            azure_lock: Arc::new(Mutex::new(())),
            m365_lock: Arc::new(Mutex::new(())),
            powershell_lock: Arc::new(Mutex::new(())),
            powershell_installed: Arc::new(Mutex::new(false)),
            aws_lock: Arc::new(Mutex::new(())),
            azure_resource_graph_lock: Arc::new(Mutex::new(())),
            github_lock: Arc::new(Mutex::new(())),
            splunk_test_lock: Arc::new(Mutex::new(())),
            threagile_lock: Arc::new(Mutex::new(())),
        })
    }

    /// Get secrets from KeyVault for the Application
    async fn get_secrets() -> Result<Secrets> {
        info!("Getting KeyVault secrets");
        let key_vault_name =
            env::var("KEY_VAULT_NAME").context("Getting key vault name from env:KEY_VAULT_NAME")?;

        get_keyvault_secrets(&key_vault_name)
            .await
            .context("Getting KeyVault secrets")
    }

    /// Create a splunk Client to send data with
    fn create_splunk_client(secrets: &Secrets) -> Result<Splunk> {
        info!("Creating Splunk client");

        let splunk = Splunk::new(
            secrets
                .splunk_host
                .as_ref()
                .context("Expect splunk_host secret")?,
            secrets
                .splunk_token
                .as_ref()
                .context("Expect splunk_token secret")?,
        )
        .context("Create Splunk Client")?;

        info!("Splunk Client created");
        Ok(splunk)
    }
}
