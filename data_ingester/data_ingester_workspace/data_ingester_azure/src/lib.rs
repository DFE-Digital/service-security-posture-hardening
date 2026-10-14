use anyhow::{Context, Result};
use data_ingester_azure_rest::azure_rest::AzureRest;
use data_ingester_ms_graph::ms_graph::MsGraph;
use data_ingester_ms_graph::service_principals::ServicePrincipalsMap;
use data_ingester_ms_graph::users::UsersMap;
use data_ingester_splunk::splunk::{set_ssphp_run, Splunk, SplunkTrait, ToHecEvents};
use data_ingester_splunk::splunk::{set_ssphp_run_with_seconds, try_collect_send};
use data_ingester_supporting::keyvault::Secrets;
use std::sync::Arc;
use tracing::info;

pub static SSPHP_RUN_KEY: &str = "azure";

/// Collect the Entra and Azure identity-and-access estate in one run.
///
/// Renamed from `azure_users` on 2026-10-14: it has never only been users - it already collected
/// subscriptions, Azure role definitions and assignments, Conditional Access policies, AAD role
/// definitions and the PIM assignment and eligibility schedules - and it now collects service
/// principals with their group memberships as well.
///
/// THE "azure_users" SSPHP_RUN KEY BELOW DOES NOT MOVE WITH THE RENAME. It is a data-plane
/// string: every event this run emits is stamped by looking the run id up under that exact name
/// (see `ssphp_run_key()` on the HEC impls), and the detections select the latest snapshot with
/// `stats max(SSPHP_RUN)`. Changing it would silently break those subsearches. Users and service
/// principals are one run and share one key.
pub async fn azure_iam(secrets: Arc<Secrets>, splunk: Arc<Splunk>) -> Result<()> {
    let ssphp_run = set_ssphp_run(SSPHP_RUN_KEY)?;
    set_ssphp_run_with_seconds(data_ingester_azure_rest::SSPHP_RUN_KEY, ssphp_run)?;
    set_ssphp_run_with_seconds("azure_users", ssphp_run)?;

    info!("Starting Azure IAM collection (users and service principals)");
    info!("GIT_HASH: {}", env!("GIT_HASH"));

    let ms_graph = MsGraph::new(
        secrets
            .azure_client_id
            .as_ref()
            .context("Expect azure_client_id secret")?,
        secrets
            .azure_client_secret
            .as_ref()
            .context("Expect azure_client_secret secret")?,
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
            .context("Expect azure_client_secret secret")?,
        secrets
            .azure_tenant_id
            .as_ref()
            .context("Expect azure_tenant_id secret")?,
    )
    .await?;

    info!("Azure logged in");

    let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel::<UsersMap>();

    info!("Getting Azure users");

    let ms_graph_clone = ms_graph.clone();
    let list_users = tokio::spawn(async move {
        ms_graph_clone.list_users_channel(sender).await?;
        anyhow::Ok::<()>(())
    });

    info!("Getting Azure Subscriptions");
    let subscriptions = azure_rest.azure_subscriptions().await?;
    splunk.send_batch((&subscriptions).to_hec_events()?).await?;

    info!("Getting Azure Subscription policies");
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

    info!("Getting PIM role assignment schedules");
    let mut aad_pim_role_assignment_schedules = ms_graph.list_role_assignment_schedules().await?;
    info!("Getting PIM role eligibility schedules");
    let aad_pim_role_eligibility_schedules = ms_graph.list_role_eligibility_schedules().await?;
    aad_pim_role_assignment_schedules
        .inner
        .extend(aad_pim_role_eligibility_schedules.inner);

    // The PIM schedules, the AAD role definitions and the Azure RBAC snapshot now feed TWO
    // consumers - users and service principals - so they are shared rather than moved into the
    // first one. Both need the same inputs: a role held by a GROUP, in Entra or in Azure, is
    // inherited by every principal in that group whatever kind of principal it is.
    let aad_pim_role_assignment_schedules = Arc::new(aad_pim_role_assignment_schedules);
    let aad_role_definitions = Arc::new(aad_role_definitions);
    let subscription_role_assignments = Arc::new(subscription_role_assignments);
    let subscription_role_definitions = Arc::new(subscription_role_definitions);

    info!("Getting Azure service principals");

    let (sp_sender, mut sp_receiver) =
        tokio::sync::mpsc::unbounded_channel::<ServicePrincipalsMap>();

    let ms_graph_sp_list = ms_graph.clone();
    let list_service_principals = tokio::spawn(async move {
        ms_graph_sp_list
            .list_service_principals_channel(sp_sender)
            .await?;
        anyhow::Ok::<()>(())
    });

    // WHY THIS CONSUMER EXISTS. Until 2026-10-14 there was no way to answer "which service
    // principals hold Entra privilege": `transitiveMemberOf` was collected for users only,
    // `/v1.0/groups` carries no members, and the `/v1.0/servicePrincipals` feed selects ten
    // attributes with membership not among them. So any population of privileged principals
    // silently omitted every service principal holding a role through a group - the case most
    // worth finding, since a service principal exists in order to be used.
    //
    // ORDER IS LOAD-BEARING: the memberships must be fetched BEFORE `add_pim_membership`, which
    // works by matching a role schedule's group against each principal's `transitive_member_of`.
    // Run it the other way round and every group-held role is missed in silence.
    //
    // `set_is_privileged` is deliberately NOT called here. On the user side that flag was
    // measured on 2026-10-14 as catching only 3 of the 10 principals eligible for Global
    // Administrator, so it needs its own look before it is extended to anything else.
    let ms_graph_sp = ms_graph.clone();
    let splunk_sp = splunk.clone();
    let sp_role_schedules = aad_pim_role_assignment_schedules.clone();
    let sp_role_definitions = aad_role_definitions.clone();
    let sp_azure_role_assignments = subscription_role_assignments.clone();
    let sp_azure_role_definitions = subscription_role_definitions.clone();
    let process_service_principals_to_splunk = tokio::spawn(async move {
        while let Some(mut service_principals) = sp_receiver.recv().await {
            ms_graph_sp
                .get_transitive_members_of_for_service_principals(&mut service_principals)
                .await?;

            service_principals.add_pim_membership(&sp_role_schedules, &sp_role_definitions);

            // Azure RBAC is where most service principal privilege actually is - measured
            // 2026-10-14 as roughly 988 service principals against 64 users, 481 of them holding
            // a role rated impact 3. Both routes are covered, direct and through a group.
            service_principals
                .add_azure_roles(&sp_azure_role_assignments, &sp_azure_role_definitions)
                .context("Failed to add azure roles to service principals")?;

            splunk_sp
                .send_batch((&service_principals).to_hec_events()?)
                .await?;
        }
        anyhow::Ok::<()>(())
    });

    let splunk_clone = splunk.clone();
    let process_to_splunk = tokio::spawn(async move {
        while let Some(mut users) = receiver.recv().await {
            ms_graph
                .get_transitive_members_of_for_users(&mut users)
                .await?;
            users.add_pim_membership(&aad_pim_role_assignment_schedules, &aad_role_definitions);

            users
                .add_azure_roles(
                    &subscription_role_assignments,
                    &subscription_role_definitions,
                )
                .context("Failed to add azure roles")?;

            users.set_is_privileged(&aad_role_definitions);

            users.process_caps(&caps);

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

    let _ = try_collect_send(
        "Azure Security SQL Firewall Rules",
        azure_rest.get_microsoft_sql_encryption_firewall_rules(),
        &splunk,
    )
    .await;

    let _ = try_collect_send(
        "Azure Security SQL server Auditing",
        azure_rest.get_microsoft_sql_server_auditing(),
        &splunk,
    )
    .await;

    let _ = try_collect_send(
        "Azure Security SQL database Auditing",
        azure_rest.get_microsoft_sql_database_auditing(),
        &splunk,
    )
    .await;

    let _ = list_users.await?;

    let _ = list_service_principals.await?;

    let _ = process_to_splunk.await?;

    let _ = process_service_principals_to_splunk.await?;

    Ok(())
}
