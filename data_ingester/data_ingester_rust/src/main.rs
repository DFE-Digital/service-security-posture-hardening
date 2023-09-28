mod admin_request_consent_policy;
mod azure_functions;
mod conditional_access_policies;
mod directory_roles;
mod groups;
mod keyvault;
mod ms_graph;
mod powershell;
mod roles;
mod security_score;
mod splunk;
mod users;

use anyhow::Result;
use azure_functions::start_server;
use tokio::sync::oneshot;

#[tokio::main]
async fn main() -> Result<()> {
    eprintln!("Starting Data Ingester...");
    let (tx, rx) = oneshot::channel::<()>();
    start_server(tx).await?;
    let _ = rx.await;
    eprintln!("Warp server started...");
    Ok(())
}
