use crate::app_state::AppState;
use crate::app_state::AppStateHealthCheck;
use crate::azure_request_response::AzureInvokeRequest;
use crate::azure_request_response::AzureInvokeResponse;
use crate::runner::function_runner;
use crate::start_local_tracing;
use anyhow::Context;
use anyhow::Result;
use axum::http::HeaderMap;
use axum::{extract::State, routing::get, routing::post, Json, Router};
use data_ingester_splunk::splunk::set_ssphp_run;
use data_ingester_splunk::start_splunk_tracing;
use std::env;
use std::sync::Arc;
use tokio::sync::oneshot::Sender;
use tracing::info;
use tracing::trace;
use valuable::Valuable;

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
    let name = "axum_server";
    let tracing_guard = start_local_tracing().context("Starting Tracing for server pre Splunk")?;

    info!(name = name, "Starting");

    let app_state = AppState::new().await.context("Building App State")?;

    set_ssphp_run("default")?;
    start_splunk_tracing(
        app_state.splunk.clone(),
        "data_ingester_rust",
        "data_ingester_rust",
    )
    .context("Start Splunk Tracing")?;

    drop(tracing_guard);
    info!(name = name, "Splunk tracing started");

    let app = Router::new()
        .route("/", get(get_root))
        .route("/healthcheck", get(get_health_check))
        .route("/healthcheck", post(get_health_check))
        .route("/aws", post(post_aws))
        .route("/azure", post(post_azure))
        .route("/azure_dev_ops", post(post_azure_dev_ops))
        .route("/azure_resource_graph", post(post_azure_resource_graph))
        .route("/github", post(post_github))
        .route(
            "/github_custom_properties",
            post(post_github_custom_properties),
        )
        .route("/m365", post(post_m365))
        .route(
            "/financial_business_partners",
            post(post_financial_business_partners),
        )
        .route("/powershell", post(post_powershell))
        .route("/qualys_qvs", post(post_qualys_qvs))
        .route("/sonar_cloud", post(post_sonar_cloud))
        .route("/threagile", post(post_threagile))
        .with_state(Arc::new(app_state));

    let port_key = "FUNCTIONS_CUSTOMHANDLER_PORT";
    let port: u16 = match env::var(port_key) {
        Ok(val) => val.parse().expect("Custom Handler port is not a number!"),
        Err(_) => 3000,
    };

    info!(name = name, port = port);
    let listener = tokio::net::TcpListener::bind(("0.0.0.0", port))
        .await
        .context("Binding to socket")?;
    let axum_serve = axum::serve(listener, app);
    tx.send(())
        .expect("Caller should be listening for Warp start event");
    axum_serve.await.context("Axum Serv")?;
    Ok(())
}

/// Health check
async fn get_root(headers: HeaderMap) -> Json<AzureInvokeResponse> {
    trace!("root request");
    Json(AzureInvokeResponse {
        outputs: None,
        logs: vec![
            "GET /".to_string(),
            format!("GIT_HASH: {}", env!("GIT_HASH")),
            format!("Headers: {:?}", headers),
        ],
        return_value: None,
    })
}

/// Health check
async fn get_health_check(
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
) -> Json<AzureInvokeResponse> {
    trace!("Health check");
    let stats = state.stats.read().await;
    let app_state_health_check = AppStateHealthCheck::from((&state, &(*stats)));

    info!(health_state = app_state_health_check.as_value(), headers=?headers);

    let app_state_health_check_json = serde_json::to_string(&app_state_health_check)
        .unwrap_or_else(|_| "ERROR converting AppState to Json".to_string());
    Json(AzureInvokeResponse {
        outputs: None,
        logs: vec![
            "Health Check".to_string(),
            format!("GIT_HASH: {}", env!("GIT_HASH")),
            app_state_health_check_json,
            format!("Headers: {:?}", headers),
        ],
        return_value: None,
    })
}

/// Collect AWS data
#[axum::debug_handler]
async fn post_aws(
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
    payload: std::result::Result<
        Option<Json<AzureInvokeRequest>>,
        axum::extract::rejection::JsonRejection,
    >,
) -> Json<AzureInvokeResponse> {
    Json(
        function_runner(
            "AWS",
            state.aws_lock.clone(),
            state,
            data_ingester_aws::aws::aws,
            headers,
            payload.unwrap_or(None),
        )
        .await,
    )
}

/// Collect Azure data
#[axum::debug_handler]
async fn post_azure(
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
    payload: std::result::Result<
        Option<Json<AzureInvokeRequest>>,
        axum::extract::rejection::JsonRejection,
    >,
) -> Json<AzureInvokeResponse> {
    Json(
        function_runner(
            "Azure",
            state.azure_lock.clone(),
            state,
            data_ingester_azure::azure_users,
            headers,
            payload.unwrap_or(None),
        )
        .await,
    )
}

/// Collect Azure Resource Graph data
#[axum::debug_handler]
async fn post_azure_resource_graph(
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
    payload: std::result::Result<
        Option<Json<AzureInvokeRequest>>,
        axum::extract::rejection::JsonRejection,
    >,
) -> Json<AzureInvokeResponse> {
    Json(
        function_runner(
            "Azure Resource Graph",
            state.azure_resource_graph_lock.clone(),
            state,
            data_ingester_azure_rest::resource_graph::azure_resource_graph,
            headers,
            payload.unwrap_or(None),
        )
        .await,
    )
}

/// Collect GitHub data
#[axum::debug_handler]
async fn post_github(
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
    payload: std::result::Result<
        Option<Json<AzureInvokeRequest>>,
        axum::extract::rejection::JsonRejection,
    >,
) -> Json<AzureInvokeResponse> {
    Json(
        function_runner(
            "GitHub",
            state.github_lock.clone(),
            state,
            data_ingester_github::entrypoint::github_octocrab_entrypoint,
            headers,
            payload.unwrap_or(None),
        )
        .await,
    )
}

/// Set GitHub Custom Properties
#[axum::debug_handler]
async fn post_github_custom_properties(
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
    payload: std::result::Result<
        Option<Json<AzureInvokeRequest>>,
        axum::extract::rejection::JsonRejection,
    >,
) -> Json<AzureInvokeResponse> {
    Json(
        function_runner(
            "GitHub Custom Properties",
            state.github_custom_properties_lock.clone(),
            state,
            data_ingester_github::entrypoint::github_set_custom_properties_entrypoint,
            headers,
            payload.unwrap_or(None),
        )
        .await,
    )
}

/// Collect M365 data
#[axum::debug_handler]
async fn post_m365(
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
    payload: std::result::Result<
        Option<Json<AzureInvokeRequest>>,
        axum::extract::rejection::JsonRejection,
    >,
) -> Json<AzureInvokeResponse> {
    Json(
        function_runner(
            "M365",
            state.m365_lock.clone(),
            state,
            data_ingester_ms_graph::ms_graph::m365,
            headers,
            payload.unwrap_or(None),
        )
        .await,
    )
}

/// Collect Powershell data
///
/// Installs powershell on the functions host before collection
#[axum::debug_handler]
async fn post_powershell(
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
    payload: std::result::Result<
        Option<Json<AzureInvokeRequest>>,
        axum::extract::rejection::JsonRejection,
    >,
) -> Json<AzureInvokeResponse> {
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
            headers,
            payload.unwrap_or(None),
        )
        .await,
    )
}

/// Collect Splunk test data
#[axum::debug_handler]
async fn post_qualys_qvs(
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
    payload: std::result::Result<
        Option<Json<AzureInvokeRequest>>,
        axum::extract::rejection::JsonRejection,
    >,
) -> Json<AzureInvokeResponse> {
    Json(
        function_runner(
            "Qualys QVS",
            state.qualys_qvs_lock.clone(),
            state,
            data_ingester_qualys::entrypoint::qualys_qvs,
            headers,
            payload.unwrap_or(None),
        )
        .await,
    )
}

/// Run Threagile against assets from Splunk
#[axum::debug_handler]
async fn post_sonar_cloud(
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
    payload: std::result::Result<
        Option<Json<AzureInvokeRequest>>,
        axum::extract::rejection::JsonRejection,
    >,
) -> Json<AzureInvokeResponse> {
    Json(
        function_runner(
            "Sonar Cloud",
            state.sonar_cloud.clone(),
            state,
            data_ingester_sonar_cloud::entrypoint,
            headers,
            payload.unwrap_or(None),
        )
        .await,
    )
}

/// Run Threagile against assets from Splunk
#[axum::debug_handler]
async fn post_threagile(
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
    payload: std::result::Result<
        Option<Json<AzureInvokeRequest>>,
        axum::extract::rejection::JsonRejection,
    >,
) -> Json<AzureInvokeResponse> {
    Json(
        function_runner(
            "Threagile",
            state.threagile_lock.clone(),
            state,
            data_ingester_threagile::threagile,
            headers,
            payload.unwrap_or(None),
        )
        .await,
    )
}

#[axum::debug_handler]
async fn post_financial_business_partners(
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
    payload: std::result::Result<
        Option<Json<AzureInvokeRequest>>,
        axum::extract::rejection::JsonRejection,
    >,
) -> Json<AzureInvokeResponse> {
    Json(
        function_runner(
            "Financial Business Partners",
            state.financial_business_partners_lock.clone(),
            state,
            data_ingester_financial_business_partners::entrypoint,
            headers,
            payload.unwrap_or(None),
        )
        .await,
    )
}

#[axum::debug_handler]
async fn post_azure_dev_ops(
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
    payload: std::result::Result<
        Option<Json<AzureInvokeRequest>>,
        axum::extract::rejection::JsonRejection,
    >,
) -> Json<AzureInvokeResponse> {
    Json(
        function_runner(
            "Azure Dev Ops",
            state.azure_dev_ops_lock.clone(),
            state,
            data_ingester_azure_dev_ops::entrypoint::entrypoint,
            headers,
            payload.unwrap_or(None),
        )
        .await,
    )
}
