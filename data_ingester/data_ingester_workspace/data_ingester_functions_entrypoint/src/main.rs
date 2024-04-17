mod azure_functions;
use std::thread::available_parallelism;

use anyhow::{Context, Result};
use azure_functions::start_server;
use memory_stats::memory_stats;
use tokio::sync::oneshot;
use tracing::{debug, info, instrument, warn};
use tracing_subscriber::{layer::SubscriberExt, EnvFilter, Registry};

#[tokio::main(flavor = "multi_thread")]
#[instrument]
async fn main() -> Result<()> {
    println!("Starting tracing ...");
    let tracing_guard = start_local_tracing();

    info!("Starting Data Ingester...");
    info!(
        "RUST_BACKTRACE={}",
        &std::env::var("RUST_BACKTRACE").unwrap_or_else(|_| "NO VALUE SET".to_string())
    );
    info!(
        "RUST_LOG={}",
        &std::env::var("RUST_LOG").unwrap_or_else(|_| "NO VALUE SET".to_string())
    );    
    let (tx, rx) = oneshot::channel::<()>();
    let server = tokio::spawn(start_server(tx));
//    drop(_tracing_before_splunk);
    let _ = rx.await;
    info!("Warp server started...");
    if let Some(usage) = memory_stats() {
        debug!(
            "Current physical memory usage: {}",
            usage.physical_mem / 1_000_000
        );
        debug!(
            "Current virtual memory usage: {}",
            usage.virtual_mem / 1_000_000
        );
    } else {
        warn!("Couldn't get the current memory usage :(");
    }
    server.await??;
    if let Some(usage) = memory_stats() {
        debug!(
            "Current physical memory usage: {}",
            usage.physical_mem / 1_000_000
        );
        debug!(
            "Current virtual memory usage: {}",
            usage.virtual_mem / 1_000_000
        );
    } else {
        warn!("Couldn't get the current memory usage :(");
    }
    Ok(())
}

pub(crate) fn start_local_tracing() -> () {
    let stdout_log = tracing_subscriber::fmt::layer().pretty();
    let subscriber = Registry::default().with(stdout_log).with(EnvFilter::from_default_env());
    tracing::subscriber::set_default(subscriber)
}
