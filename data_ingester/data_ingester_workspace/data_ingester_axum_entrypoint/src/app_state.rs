use anyhow::Context;
use anyhow::Result;
use axum::http::HeaderMap;
use axum::Json;
use data_ingester_splunk::splunk::{Splunk, SplunkTrait};
use data_ingester_supporting::keyvault::get_keyvault_secrets;
use data_ingester_supporting::keyvault::Secrets;
use serde::Serialize;
use std::collections::HashMap;
use std::env;
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::Mutex;
use tokio::sync::RwLock;
use tracing::info;
use valuable::Valuable;

use crate::azure_request_response::AzureInvokeRequest;

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

    /// Lock for azure_resource_graph to stop concurrent executions
    pub(crate) azure_dev_ops_lock: Arc<Mutex<()>>,

    /// Lock for Financial Business Partners
    pub(crate) financial_business_partners_lock: Arc<Mutex<()>>,

    /// Lock for github to stop concurrent executions
    pub(crate) github_lock: Arc<Mutex<()>>,

    /// Lock for github_custom_properties to stop concurrent executions
    pub(crate) github_custom_properties_lock: Arc<Mutex<()>>,

    /// Lock for m365 to stop concurrent executions
    pub(crate) m365_lock: Arc<Mutex<()>>,

    /// Lock for powershell to stop concurrent executions
    pub(crate) powershell_lock: Arc<Mutex<()>>,
    /// Is powershell installed in our function?
    pub(crate) powershell_installed: Arc<Mutex<bool>>,

    /// Lock for splunk_test to stop concurrent executions
    pub(crate) sonar_cloud: Arc<Mutex<()>>,

    /// Lock for qualys_qvs to stop concurrent executions
    pub(crate) qualys_qvs_lock: Arc<Mutex<()>>,

    /// Lock for threagile to stop concurrent executions
    pub(crate) threagile_lock: Arc<Mutex<()>>,
    pub(crate) stats: Arc<RwLock<Stats>>,
}

/// Records stats for requsets made to this execution
#[derive(Serialize, Debug, Valuable)]
pub(crate) struct Stats {
    start_up_time: u64,
    instance_requests: Vec<Invocation>,
}

impl Stats {
    /// Create a new Stats struct
    fn new() -> Self {
        Self {
            start_up_time: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            instance_requests: vec![],
        }
    }

    /// Add a new Invocation
    pub(crate) fn new_invocation<T: Into<String>>(
        &mut self,
        name: T,
        headers: HeaderMap,
        request: Option<Json<AzureInvokeRequest>>,
    ) -> usize {
        self.instance_requests
            .push(Invocation::new(name, headers, request));
        self.instance_requests.len()
    }

    /// Get a Invocation by index
    pub(crate) fn get(&mut self, index: usize) -> &mut Invocation {
        &mut self.instance_requests[index]
    }
}

/// Represents a single Invocation and it's metadata
#[derive(Serialize, Debug, Valuable)]
pub(crate) struct Invocation {
    name: String,
    start: u64,
    finish: Option<u64>,
    got_lock: Option<bool>,
    errors: Option<String>,
    headers: Headers,
    request: Option<AzureInvokeRequest>,
}

impl Invocation {
    /// Create a new invocation
    pub(crate) fn new<N: Into<String>>(
        name: N,
        headers: HeaderMap,
        request: Option<Json<AzureInvokeRequest>>,
    ) -> Self {
        let headers = Headers::from(headers);
        let request = request.map(|json| json.0);
        Self {
            name: name.into(),
            start: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            finish: None,
            errors: None,
            got_lock: None,
            headers,
            request,
        }
    }

    /// Add a finish timestamp to this invocation
    pub(crate) fn finish(&mut self) {
        self.finish = Some(
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        );
    }

    /// Did this Invocation successfully get the lock
    pub(crate) fn lock_state(&mut self, state: bool) -> &mut Self {
        self.got_lock = Some(state);
        self
    }

    /// Any errors during exectuction
    pub(crate) fn errors<T: Into<String>>(&mut self, errors: T) -> &mut Self {
        self.errors = Some(errors.into());
        self
    }
}

#[derive(Serialize, Debug, Valuable)]
pub(crate) struct Headers(HashMap<String, String>);

impl From<HeaderMap> for Headers {
    fn from(value: HeaderMap) -> Self {
        let headers = value
            .iter()
            .map(|(k, v)| {
                (
                    k.as_str().to_string(),
                    v.to_str()
                        .map(|s| s.to_string())
                        .unwrap_or_else(|e| format!("{}", e)),
                )
            })
            .collect::<HashMap<String, String>>();
        Self(headers)
    }
}

#[derive(Serialize, Debug, Valuable)]
pub(crate) struct AppStateHealthCheck<'a> {
    splunk: ArcState,
    secrets: ArcState,
    aws_lock: ArcMutexState,
    azure_lock: ArcMutexState,
    azure_resource_graph_lock: ArcMutexState,
    financial_business_partners_lock: ArcMutexState,
    github_lock: ArcMutexState,
    github_custom_properties_lock: ArcMutexState,
    m365_lock: ArcMutexState,
    powershell_installed: ArcMutexState,
    powershell_lock: ArcMutexState,
    qualys_qvs_lock: ArcMutexState,
    sonar_cloud: ArcMutexState,
    threagile_lock: ArcMutexState,
    execution_stats: &'a Stats,
}

/// Records the stats for an Arc
#[derive(Serialize, Debug, Valuable)]
struct ArcState {
    arc_strong_count: usize,
    arc_weak_count: usize,
}

/// Records the stats for an Mutex
#[derive(Serialize, Debug, Valuable)]
struct MutexState {
    state: String,
}

impl<T> From<&Arc<Mutex<T>>> for MutexState {
    fn from(value: &Arc<Mutex<T>>) -> Self {
        let state = match value.try_lock() {
            Ok(_) => "Unlocked",
            Err(_) => "WouldBlock",
        };
        Self {
            state: state.to_string(),
        }
    }
}

impl<T> From<&Arc<T>> for ArcState {
    fn from(value: &Arc<T>) -> Self {
        ArcState {
            arc_strong_count: Arc::strong_count(value),
            arc_weak_count: Arc::weak_count(value),
        }
    }
}

/// Records the stats for an Arc<Mutex<T>>
#[derive(Serialize, Debug, Valuable)]
struct ArcMutexState {
    mutex: MutexState,
    arc: ArcState,
}

impl<T> From<&Arc<Mutex<T>>> for ArcMutexState {
    fn from(value: &Arc<Mutex<T>>) -> Self {
        ArcMutexState {
            arc: value.into(),
            mutex: MutexState::from(value),
        }
    }
}

impl<'a, 'b> From<(&'b Arc<AppState>, &'a Stats)> for AppStateHealthCheck<'a> {
    fn from((value, stats): (&'b Arc<AppState>, &'a Stats)) -> Self {
        Self {
            splunk: (&value.splunk).into(),
            secrets: (&value.secrets).into(),
            aws_lock: (&value.aws_lock).into(),
            azure_lock: (&value.azure_lock).into(),
            azure_resource_graph_lock: (&value.azure_resource_graph_lock).into(),
            financial_business_partners_lock: (&value.financial_business_partners_lock).into(),
            github_lock: (&value.github_lock).into(),
            github_custom_properties_lock: (&value.github_custom_properties_lock).into(),
            m365_lock: (&value.m365_lock).into(),
            powershell_installed: (&value.powershell_installed).into(),
            powershell_lock: (&value.powershell_lock).into(),
            sonar_cloud: (&value.sonar_cloud).into(),
            qualys_qvs_lock: (&value.qualys_qvs_lock).into(),
            threagile_lock: (&value.threagile_lock).into(),
            execution_stats: stats,
        }
    }
}

impl AppState {
    /// Create a new AppState
    pub(crate) async fn new() -> Result<Self> {
        let secrets = AppState::get_secrets().await?;
        let splunk = AppState::create_splunk_client(&secrets)?;
        Ok(Self {
            secrets: Arc::new(secrets),
            splunk: Arc::new(splunk),

            aws_lock: Arc::new(Mutex::new(())),
            azure_lock: Arc::new(Mutex::new(())),
            azure_resource_graph_lock: Arc::new(Mutex::new(())),
            azure_dev_ops_lock: Arc::new(Mutex::new(())),
            financial_business_partners_lock: Arc::new(Mutex::new(())),
            github_lock: Arc::new(Mutex::new(())),
            github_custom_properties_lock: Arc::new(Mutex::new(())),
            m365_lock: Arc::new(Mutex::new(())),
            powershell_installed: Arc::new(Mutex::new(false)),
            powershell_lock: Arc::new(Mutex::new(())),
            qualys_qvs_lock: Arc::new(Mutex::new(())),
            sonar_cloud: Arc::new(Mutex::new(())),
            stats: Arc::new(RwLock::new(Stats::new())),
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
        let hec_acknowledgement = false;
        let splunk = Splunk::new(
            secrets
                .splunk_host
                .as_ref()
                .context("Expect splunk_host secret")?,
            secrets
                .splunk_token
                .as_ref()
                .context("Expect splunk_token secret")?,
            hec_acknowledgement,
        )
        .context("Create Splunk Client")?;

        info!("Splunk Client created");
        Ok(splunk)
    }
}
