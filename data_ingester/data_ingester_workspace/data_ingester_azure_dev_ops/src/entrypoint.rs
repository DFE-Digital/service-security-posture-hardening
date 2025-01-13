use anyhow::{Context, Result};
use data_ingester_splunk::splunk::{Splunk, set_ssphp_run, try_collect_send};
use data_ingester_supporting::keyvault::Secrets;
use std::sync::Arc;
use tracing::info;

use crate::ado_dev_ops_client::AzureDevOpsClient;

pub async fn entrypoint(secrets: Arc<Secrets>, splunk: Arc<Splunk>) -> Result<()> {
    set_ssphp_run("github")?;
    info!("Starting Azure DevOps ADO collection");

    let client_id = secrets
        .azure_client_id
        .as_ref()
        .context("No Azure Client Id")?;

    let client_secret = secrets
        .azure_client_secret
        .as_ref()
        .context("No Azure Client Secret")?;

    let tenant_id = secrets
        .azure_tenant_id
        .as_ref()
        .context("No Azure Tenant Id")?;

    let ado = AzureDevOpsClient::new(client_id, client_secret, tenant_id)
        .await
        .context("Building AzureDevOpsClient")?;

    let org_name = "foo";

    let _organizations = try_collect_send(
        &format!("Azure DevOps Organizations for tenant: {}", tenant_id),
        ado.organizations_list(),
        &splunk,
    )
    .await?;

    let _projects = try_collect_send(
        &format!("Projects for {org_name}"),
        ado.projects_list(org_name),
        &splunk,
    )
    .await?;

    let _ = try_collect_send(
        &format!("Git Policy Configuration for {org_name}"),
        ado.git_policy_configuration_get(org_name, "foo"),
        &splunk,
    )
    .await?;

    let _ = try_collect_send(
        &format!("Git repository list {org_name}"),
        ado.git_repository_list(org_name, "foo"),
        &splunk,
    )
    .await?;

    Ok(())
}
