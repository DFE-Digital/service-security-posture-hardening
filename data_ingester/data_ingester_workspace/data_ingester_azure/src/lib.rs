use anyhow::{Context, Result};
use data_ingester_azure_rest::azure_rest::AzureRest;
use data_ingester_ms_graph::ms_graph::MsGraph;
use data_ingester_ms_graph::users::UsersMap;
use data_ingester_splunk::splunk::try_collect_send;
use data_ingester_splunk::splunk::{set_ssphp_run, Splunk, ToHecEvents, SplunkTrait};
use data_ingester_supporting::keyvault::Secrets;
use std::sync::Arc;
use tracing::info;

pub async fn azure_users(secrets: Arc<Secrets>, splunk: Arc<Splunk>) -> Result<()> {
    set_ssphp_run("azure_users")?;

    info!("Starting Azure Users collection");
    info!("GIT_HASH: {}", env!("GIT_HASH"));

    let ms_graph = MsGraph::new(
        secrets
            .azure_client_id
            .as_ref()
            .context("Expect azure_client_id secret")?,
        secrets
            .azure_client_secret
            .as_ref()
            .context("Expect azure_secret_id secret")?,
        secrets
            .azure_tenant_id
            .as_ref()
            .context("Expect azure_tenant_id secret")?,
    )
    .await?;

    let azure_rest = AzureRest::new(
        secrets
            .azure_client_id
            .as_ref()
            .context("Expect azure_client_id secret")?,
        secrets
            .azure_client_secret
            .as_ref()
            .context("Expect azure_secret_id secret")?,
        secrets
            .azure_tenant_id
            .as_ref()
            .context("Expect azure_tenant_id secret")?,
    )
    .await?;

    info!("Azure logged in");

    let (sender, mut reciever) = tokio::sync::mpsc::unbounded_channel::<UsersMap>();

    info!("Getting Azure users");

    let ms_graph_clone = ms_graph.clone();
    let list_users = tokio::spawn(async move {
        ms_graph_clone.list_users_channel(sender).await?;
        anyhow::Ok::<()>(())
    });

    info!("Getting Azure Subscriptions");
    let subscriptions = azure_rest.azure_subscriptions().await?;
    splunk.send_batch((&subscriptions).to_hec_events()?).await?;

    info!("Getting Azure Subscriptions");
    let subscription_policies = azure_rest.get_microsoft_subscription_policies().await?;
    splunk
        .send_batch((&subscription_policies).to_hec_events()?)
        .await?;

    info!("Getting Azure Subscription RoleDefinitions");
    let subscription_role_definitions = azure_rest.azure_role_definitions().await?;
    splunk
        .send_batch((&subscription_role_definitions).to_hec_events()?)
        .await?;

    info!("Getting Azure Subscription RoleAssignments");
    let subscription_role_assignments = azure_rest.azure_role_assignments().await?;
    splunk
        .send_batch((&subscription_role_assignments).to_hec_events()?)
        .await?;

    info!("Getting AAD Conditional access policies");
    let caps = ms_graph.list_conditional_access_policies().await?;
    splunk.send_batch((&caps).to_hec_events()?).await?;

    info!("Getting AAD roles definitions");
    let aad_role_definitions = ms_graph.list_role_definitions().await?;
    splunk
        .send_batch((&aad_role_definitions).to_hec_events()?)
        .await?;

    let splunk_clone = splunk.clone();
    let process_to_splunk = tokio::spawn(async move {
        while let Some(mut users) = reciever.recv().await {
            users.set_is_privileged(&aad_role_definitions);

            users.process_caps(&caps);

            users
                .add_azure_roles(
                    &subscription_role_assignments,
                    &subscription_role_definitions,
                )
                .context("Failed to add azure roles")?;

            splunk_clone.send_batch((&users).to_hec_events()?).await?;
        }
        anyhow::Ok::<()>(())
    });

    let _ = try_collect_send(
        "Azure Security Contacts",
        azure_rest.get_security_contacts(),
        &splunk,
    )
    .await;

    let _ = try_collect_send(
        "Azure Security Center built in",
        azure_rest.get_security_center_built_in(),
        &splunk,
    )
    .await;

    let _ = try_collect_send(
        "Azure Security Auto Provisioning Settings",
        azure_rest.get_microsoft_security_auto_provisioning_settings(),
        &splunk,
    )
    .await;

    let _ = try_collect_send(
        "Azure Security Settings",
        azure_rest.get_microsoft_security_settings(),
        &splunk,
    )
    .await;

    let _ = try_collect_send(
        "Azure Security SQL Encryption protection",
        azure_rest.get_microsoft_sql_encryption_protection(),
        &splunk,
    )
    .await;

    let _ = list_users.await?;

    let _ = process_to_splunk.await?;

    Ok(())
}
