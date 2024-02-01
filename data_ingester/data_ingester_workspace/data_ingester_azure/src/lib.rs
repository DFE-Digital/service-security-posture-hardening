use anyhow::{Context, Result};
use data_ingester_azure_rest::azure_rest::AzureRest;
use data_ingester_ms_graph::ms_graph::login;
use data_ingester_ms_graph::users::UsersMap;
use data_ingester_splunk::splunk::try_collect_send;
use data_ingester_splunk::splunk::{set_ssphp_run, Splunk, ToHecEvents};
use data_ingester_supporting::keyvault::Secrets;
use std::sync::Arc;

pub async fn azure_users(secrets: Arc<Secrets>, splunk: Arc<Splunk>) -> Result<()> {
    set_ssphp_run()?;

    splunk.log("Starting Azure Users collection").await?;
    splunk
        .log(&format!("GIT_HASH: {}", env!("GIT_HASH")))
        .await?;

    let ms_graph = login(
        &secrets.azure_client_id,
        &secrets.azure_client_secret,
        &secrets.azure_tenant_id,
    )
    .await?;

    let azure_rest = AzureRest::new(
        &secrets.azure_client_id,
        &secrets.azure_client_secret,
        &secrets.azure_tenant_id,
    )
    .await?;

    splunk.log("Azure logged in").await?;

    let (sender, mut reciever) = tokio::sync::mpsc::unbounded_channel::<UsersMap>();

    splunk.log("Getting Azure users").await?;

    let splunk_clone = splunk.clone();
    let ms_graph_clone = ms_graph.clone();
    let list_users = tokio::spawn(async move {
        ms_graph_clone
            .list_users_channel(&splunk_clone, sender)
            .await?;
        anyhow::Ok::<()>(())
    });

    let ms_graph_clone = ms_graph.clone();
    let splunk_clone = splunk.clone();

    splunk.log("Getting Azure Subscriptions").await?;
    let subscriptions = azure_rest.azure_subscriptions().await?;
    splunk.send_batch((&subscriptions).to_hec_events()?).await?;

    splunk.log("Getting Azure Subscriptions").await?;
    let subscription_policies = azure_rest.get_microsoft_subscription_policies().await?;
    splunk
        .send_batch((&subscription_policies).to_hec_events()?)
        .await?;

    splunk
        .log("Getting Azure Subscription RoleDefinitions")
        .await?;
    let subscription_role_definitions = azure_rest.azure_role_definitions().await?;
    splunk
        .send_batch((&subscription_role_definitions).to_hec_events()?)
        .await?;

    splunk
        .log("Getting Azure Subscription RoleAssignments")
        .await?;
    let subscription_role_assignments = azure_rest.azure_role_assignments().await?;
    splunk
        .send_batch((&subscription_role_assignments).to_hec_events()?)
        .await?;

    splunk
        .log("Getting AAD Conditional access policies")
        .await?;
    let caps = ms_graph.list_conditional_access_policies().await?;
    splunk.send_batch((&caps).to_hec_events()?).await?;

    splunk.log("Getting AAD roles definitions").await?;
    let aad_role_definitions = ms_graph.list_role_definitions().await?;
    splunk
        .send_batch((&aad_role_definitions).to_hec_events()?)
        .await?;

    try_collect_send(
        "Azure Security Contacts",
        azure_rest.get_security_contacts(),
        &splunk,
    )
    .await?;

    try_collect_send(
        "Azure Security Center built in",
        azure_rest.get_security_center_built_in(),
        &splunk,
    )
    .await?;

    try_collect_send(
        "Azure Security Auto Provisioning Settings",
        azure_rest.get_microsoft_security_auto_provisioning_settings(),
        &splunk,
    )
    .await?;

    try_collect_send(
        "Azure Security Settings",
        azure_rest.get_microsoft_security_settings(),
        &splunk,
    )
    .await?;

    try_collect_send(
        "Azure Security SQL Encryption protection",
        azure_rest.get_microsoft_sql_encryption_protection(),
        &splunk,
    )
    .await?;

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

            splunk.send_batch((&users).to_hec_events()?).await?;
        }
        anyhow::Ok::<()>(())
    });

    let admin_request_consent_policy = ms_graph_clone
        .get_admin_request_consent_policy()
        .await
        .unwrap();

    splunk_clone
        .send_batch((&admin_request_consent_policy).to_hec_events().unwrap())
        .await?;

    let _ = list_users.await?;

    let _ = process_to_splunk.await?;

    Ok(())
}
