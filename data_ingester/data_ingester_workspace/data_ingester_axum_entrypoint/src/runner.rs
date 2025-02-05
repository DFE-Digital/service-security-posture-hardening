use axum::{http::HeaderMap, Json};
use data_ingester_splunk::splunk::Splunk;
use data_ingester_supporting::keyvault::Secrets;
use std::{future::Future, sync::Arc};
use tokio::{
    sync::Mutex,
    time::{timeout, Duration},
};
use tracing::{error, info, instrument};

use crate::{
    app_state::AppState,
    azure_request_response::{AzureInvokeRequest, AzureInvokeResponse},
};

enum RunnerState {
    Start,
    Lock,
    Running,
    Complete,
}

impl RunnerState {
    fn new() -> Self {
        Self::Start
    }

    fn next(&mut self) {
        *self = match self {
            RunnerState::Start => RunnerState::Lock,
            RunnerState::Lock => RunnerState::Running,
            RunnerState::Running => RunnerState::Complete,
            RunnerState::Complete => RunnerState::Complete,
        }
    }
}

impl std::fmt::Display for RunnerState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let stage = match self {
            RunnerState::Start => "Start",
            RunnerState::Lock => "Lock",
            RunnerState::Running => "Running",
            RunnerState::Complete => "Complete",
        };
        write!(f, "{}", stage)
    }
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
#[instrument(skip(lock, state, func, headers, request))]
pub(crate) async fn function_runner<F, R>(
    name: &str,
    lock: Arc<Mutex<()>>,
    state: Arc<AppState>,
    func: F,
    headers: HeaderMap,
    request: Option<Json<AzureInvokeRequest>>,
) -> AzureInvokeResponse
where
    F: Fn(Arc<Secrets>, Arc<Splunk>) -> R,
    R: Future<Output = Result<(), anyhow::Error>>,
{
    let mut stage = RunnerState::new();
    info!(name= name, stage=%stage);
    let invocation_index = state
        .stats
        .write()
        .await
        .new_invocation(name, headers, request)
        - 1;

    let mut response = AzureInvokeResponse {
        outputs: None,
        logs: vec![name.to_string(), format!("GIT_HASH: {}", env!("GIT_HASH"))],
        return_value: None,
    };

    stage.next();

    let lock = match lock.try_lock() {
        Ok(lock) => {
            let _ = state
                .stats
                .write()
                .await
                .get(invocation_index)
                .lock_state(true);

            let msg = format!("{} lock aquired, starting", name);
            info!(name= name, stage=%stage, lock_aquired=true);
            response.logs.push(msg.to_owned());
            lock
        }
        Err(_) => {
            state
                .stats
                .write()
                .await
                .get(invocation_index)
                .lock_state(false)
                .finish();

            let msg = format!("{} collection is already in progress. NOT starting.", name);
            error!(name = name, stage=%stage, lock_aquired=false);
            response.logs.push(msg.to_owned());

            return response;
        }
    };
    stage.next();

    // TODO set per collector timeouts
    let result = match timeout(
        Duration::from_secs(60 * 60 * 8),
        func(state.secrets.clone(), state.splunk.clone()),
    )
    .await
    {
        Ok(result) => {
            info!(name=name, stage=%stage, complete=false, "");
            Some(result)
        }
        Err(err) => {
            state
                .stats
                .write()
                .await
                .get(invocation_index)
                .errors(format!("{:#?}", err))
                .finish();
            stage.next();
            error!(name=name, stage=%stage, complete=false, error=?err);
            None
        }
    };

    let (result, complete) = if let Some(result) = result {
        match result {
            Ok(_) => {
                stage.next();
                state.stats.write().await.get(invocation_index).finish();
                (format!("{} Success", name), true)
            }
            Err(e) => {
                state
                    .stats
                    .write()
                    .await
                    .get(invocation_index)
                    .errors(format!("{:#?}", e))
                    .finish();
                stage.next();
                error!(name=name, stage=%stage, complete=false, error=?e);
                let error = format!("{} entrypoint failed with error: {:#?}", &name, &e);
                (error, false)
            }
        }
    } else {
        let error = format!("{} entrypoint failed after collector timeout", &name);
        (error, false)
    };

    drop(lock);

    response.logs.push(result);
    error!(name=name, stage=%stage, complete=complete);
    response
}
