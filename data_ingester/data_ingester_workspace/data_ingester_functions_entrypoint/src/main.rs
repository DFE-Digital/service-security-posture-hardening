mod azure_functions;
use anyhow::Result;
use azure_functions::start_server;
use memory_stats::memory_stats;
use tokio::sync::oneshot;

#[tokio::main]
async fn main() -> Result<()> {
    eprintln!("Starting Data Ingester...");
    eprintln!(
        "RUST_BACKTRACE={}",
        &std::env::var("RUST_BACKTRACE").unwrap_or_else(|_| "NO VALUE SET".to_string())
    );
    let (tx, rx) = oneshot::channel::<()>();
    let server = tokio::spawn(start_server(tx));
    let _ = rx.await;
    eprintln!("Warp server started...");
    if let Some(usage) = memory_stats() {
        println!(
            "Current physical memory usage: {}",
            usage.physical_mem / 1_000_000
        );
        println!(
            "Current virtual memory usage: {}",
            usage.virtual_mem / 1_000_000
        );
    } else {
        println!("Couldn't get the current memory usage :(");
    }
    server.await??;
    if let Some(usage) = memory_stats() {
        println!(
            "Current physical memory usage: {}",
            usage.physical_mem / 1_000_000
        );
        println!(
            "Current virtual memory usage: {}",
            usage.virtual_mem / 1_000_000
        );
    } else {
        println!("Couldn't get the current memory usage :(");
    }
    Ok(())
}