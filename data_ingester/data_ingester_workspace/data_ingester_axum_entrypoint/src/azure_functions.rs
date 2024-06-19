use crate::app_state::AppState;
use crate::azure_request_response::AzureInvokeResponse;
use crate::start_local_tracing;
use anyhow::Context;
use anyhow::Result;
use axum::{extract::State, routing::get, routing::post, Json, Router};
use data_ingester_splunk::splunk::set_ssphp_run;
use data_ingester_splunk::splunk::Splunk;
use data_ingester_splunk::start_splunk_tracing;
use data_ingester_supporting::keyvault::Secrets;
use std::env;
use std::future::Future;
use std::sync::Arc;
use tokio::sync::oneshot::Sender;
use tokio::sync::Mutex;
use tracing::error;
use tracing::info;

/// Start the Axum server
///
/// Binds to port 3000 or the port specified by the
/// FUNCTIONS_CUSTOMHANDLER_PORT which is set by the Azure Functions
/// runtime
///
/// This will setup splunk as a global default for tracing
///
/// tx: takes a [Sender] - used send a signal indicating the server is ready
///
pub(crate) async fn start_server(tx: Sender<()>) -> Result<()> {
    let tracing_guard = start_local_tracing().context("Starting Tracing for server pre Splunk")?;

    info!("Starting server for Azure Functions");

    let app_state = AppState::new().await.context("Building App State")?;

    set_ssphp_run("default")?;
    start_splunk_tracing(
        app_state.splunk.clone(),
        "data_ingester_rust",
        "data_ingester_rust",
    )
    .context("Start Splunk Tracing")?;

    drop(tracing_guard);
    info!("Splunk tracing started");

    let app = Router::new()
        .route("/", get(get_health_check))
        .route("/aws", post(post_aws))
        .route("/azure", post(post_azure))
        .route("/azure_resource_graph", post(post_azure_resource_graph))
        .route("/github", post(post_github))
        .route("/m365", post(post_m365))
        .route("/powershell", post(post_powershell))
        .route("/qualys_qvs", post(post_qualys_qvs))
        .route("/threagile", post(post_threagile))
        .with_state(Arc::new(app_state));

    let port_key = "FUNCTIONS_CUSTOMHANDLER_PORT";
    let port: u16 = match env::var(port_key) {
        Ok(val) => val.parse().expect("Custom Handler port is not a number!"),
        Err(_) => 3000,
    };

    let listener = tokio::net::TcpListener::bind(("0.0.0.0", port))
        .await
        .context("Binding to socket")?;
    let axum_serve = axum::serve(listener, app);
    tx.send(())
        .expect("Caller should be listening for Warp start event");
    axum_serve.await.context("Axum Serv")?;
    Ok(())
}

/// Run a entrypoint function
///
/// Checks to see if the lock can be held, then runs the async
/// function for data collection.
///
/// The lock should be freed when the function completes or the
/// function fails.
///
/// name: The name of this function to use for logging
/// lock: The lock to prevent concurrent executions
/// func: an async function taking [Arc<Secrets}] and [Arc<Splunk>]
///
async fn function_runner<F, R>(
    name: &str,
    lock: Arc<Mutex<()>>,
    state: Arc<AppState>,
    func: F,
) -> AzureInvokeResponse
where
    F: Fn(Arc<Secrets>, Arc<Splunk>) -> R,
    R: Future<Output = Result<(), anyhow::Error>>,
{
    let mut response = AzureInvokeResponse {
        outputs: None,
        logs: vec![name.to_string(), format!("GIT_HASH: {}", env!("GIT_HASH"))],
        return_value: None,
    };

    let lock = match lock.try_lock() {
        Ok(lock) => {
            let msg = format!("{} lock aquired, starting", name);
            info!("{}", &msg);
            response.logs.push(msg.to_owned());
            lock
        }
        Err(_) => {
            let msg = format!("{} collection is already in progress. NOT starting.", name);
            error!("{}", &msg);
            response.logs.push(msg.to_owned());
            return response;
        }
    };

    let result = match func(state.secrets.clone(), state.splunk.clone()).await {
        Ok(_) => format!("{} Success", name),
        Err(e) => {
            error!("{:?}", &e);
            format!("Error {}: {:?}", name, e)
        }
    };
    response.logs.push(result);
    drop(lock);
    response
}

/// Health check
async fn get_health_check() -> Json<AzureInvokeResponse> {
    info!("Health check");
    Json(AzureInvokeResponse {
        outputs: None,
        logs: vec![
            "Health Check".to_string(),
            format!("GIT_HASH: {}", env!("GIT_HASH")),
        ],
        return_value: None,
    })
}

/// Collect AWS data
async fn post_aws(State(state): State<Arc<AppState>>) -> Json<AzureInvokeResponse> {
    Json(
        function_runner(
            "Azure",
            state.aws_lock.clone(),
            state,
            data_ingester_aws::aws::aws,
        )
        .await,
    )
}

/// Collect Azure data
async fn post_azure(State(state): State<Arc<AppState>>) -> Json<AzureInvokeResponse> {
    Json(
        function_runner(
            "Azure",
            state.azure_lock.clone(),
            state,
            data_ingester_azure::azure_users,
        )
        .await,
    )
}

/// Collect Azure Resource Graph data
async fn post_azure_resource_graph(
    State(state): State<Arc<AppState>>,
) -> Json<AzureInvokeResponse> {
    Json(
        function_runner(
            "Azure Resource Graph",
            state.azure_resource_graph_lock.clone(),
            state,
            data_ingester_azure_rest::resource_graph::azure_resource_graph,
        )
        .await,
    )
}

/// Collect GitHub data
async fn post_github(State(state): State<Arc<AppState>>) -> Json<AzureInvokeResponse> {
    Json(
        function_runner(
            "GitHub",
            state.github_lock.clone(),
            state,
            data_ingester_github::entrypoint::github_octocrab_entrypoint,
        )
        .await,
    )
}

/// Collect M365 data
async fn post_m365(State(state): State<Arc<AppState>>) -> Json<AzureInvokeResponse> {
    Json(
        function_runner(
            "M365",
            state.m365_lock.clone(),
            state,
            data_ingester_ms_graph::ms_graph::m365,
        )
        .await,
    )
}

/// Collect Powershell data
///
/// Installs powershell on the functions host before collection
async fn post_powershell(State(state): State<Arc<AppState>>) -> Json<AzureInvokeResponse> {
    if state.powershell_lock.try_lock().is_ok() && !*state.powershell_installed.lock().await {
        info!("Powershell: Installing");

        data_ingester_ms_powershell::powershell::install_powershell()
            .await
            .expect("Powershell should install cleanly in the Azure Function instance");
        *state.powershell_installed.lock().await = true;
    }

    Json(
        function_runner(
            "Powershell",
            state.powershell_lock.clone(),
            state,
            data_ingester_ms_powershell::runner::powershell,
        )
        .await,
    )
}

/// Collect Splunk test data
async fn post_qualys_qvs(State(state): State<Arc<AppState>>) -> Json<AzureInvokeResponse> {
    Json(
        function_runner(
            "Splunk Test",
            state.splunk_test_lock.clone(),
            state,
            data_ingester_qualys::entrypoint::qualys_qvs,
        )
        .await,
    )
}

/// Run Threagile against assets from Splunk
async fn post_threagile(State(state): State<Arc<AppState>>) -> Json<AzureInvokeResponse> {
    Json(
        function_runner(
            "Threagile Test",
            state.threagile_lock.clone(),
            state,
            data_ingester_threagile::threagile,
        )
        .await,
    )
}
