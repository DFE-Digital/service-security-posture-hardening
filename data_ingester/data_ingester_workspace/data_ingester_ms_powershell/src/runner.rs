use anyhow::Result;
use std::sync::Arc;
use tracing::info;

use data_ingester_splunk::splunk::set_ssphp_run;

use data_ingester_splunk::splunk::try_collect_send;
use data_ingester_splunk::splunk::Splunk;
use data_ingester_supporting::keyvault::Secrets;

use crate::powershell::run_powershell_exchange_login_test;
use crate::powershell::run_powershell_get_admin_audit_log_config;
use crate::powershell::run_powershell_get_anti_phish_policy;
use crate::powershell::run_powershell_get_atp_policy_for_o365;
use crate::powershell::run_powershell_get_blocked_sender_address;
use crate::powershell::run_powershell_get_cs_teams_client_configuration;
use crate::powershell::run_powershell_get_cs_tenant_federation_configuration;
use crate::powershell::run_powershell_get_dkim_signing_config;
use crate::powershell::run_powershell_get_dlp_compliance_policy;
use crate::powershell::run_powershell_get_email_tenant_settings;
use crate::powershell::run_powershell_get_eop_protection_policy_rule;
use crate::powershell::run_powershell_get_hosted_content_filter_policy;
use crate::powershell::run_powershell_get_hosted_outbound_spam_filter_policy;
use crate::powershell::run_powershell_get_mailbox;
use crate::powershell::run_powershell_get_malware_filter_policy;
use crate::powershell::run_powershell_get_management_role_assignment;
use crate::powershell::run_powershell_get_organization_config;
use crate::powershell::run_powershell_get_owa_mailbox_policy;
use crate::powershell::run_powershell_get_protection_alert;
use crate::powershell::run_powershell_get_safe_attachment_policy;
use crate::powershell::run_powershell_get_safe_links_policy;
use crate::powershell::run_powershell_get_sharing_policy;
use crate::powershell::run_powershell_get_spoof_intelligence_insight;
use crate::powershell::run_powershell_get_transport_rule;
use crate::powershell::run_powershell_get_user_vip;

pub async fn powershell(secrets: Arc<Secrets>, splunk: Arc<Splunk>) -> Result<()> {
    set_ssphp_run(crate::SSPHP_RUN_KEY)?;

    info!("Starting M365 Powershell collection");
    info!("GIT_HASH: {}", env!("GIT_HASH"));

    // M365 V2.0 2.8
    let _ = try_collect_send(
        "Exchange Login test",
        run_powershell_exchange_login_test(&secrets),
        &splunk,
    )
    .await;

    // M365 V2.0 2.8
    let _ = try_collect_send(
        "Exchange Get Management Role Assignment",
        run_powershell_get_management_role_assignment(&secrets),
        &splunk,
    )
    .await;

    // M365 V2.0 3.6
    let _ = try_collect_send(
        "MsTeams Get Cs Tenant Federation Configuration",
        run_powershell_get_cs_tenant_federation_configuration(&secrets),
        &splunk,
    )
    .await;

    // M365 V2.0 3.7
    let _ = try_collect_send(
        "MsTeams Get Cs Teams Client Configuration",
        run_powershell_get_cs_teams_client_configuration(&secrets),
        &splunk,
    )
    .await;

    // M365 V2.0 4.13
    let _ = try_collect_send(
        "Exchange Get EOP Protection Policy Rule",
        run_powershell_get_eop_protection_policy_rule(&secrets),
        &splunk,
    )
    .await;

    // Azure 365 V2.0 5.3
    // let _ = try_collect_send(
    //     "Exchange Get Mailboxes",
    //     run_powershell_get_mailbox(&secrets),
    //     &splunk,
    // )
    // .await;

    let _ = try_collect_send(
        "Exchange Get VIP Users",
        run_powershell_get_user_vip(&secrets),
        &splunk,
    )
    .await;

    let _ = try_collect_send(
        "Exchange Get Protection Alerts",
        run_powershell_get_protection_alert(&secrets),
        &splunk,
    )
    .await;

    let _ = try_collect_send(
        "Exchange Get Email Tenant Settings",
        run_powershell_get_email_tenant_settings(&secrets),
        &splunk,
    )
    .await;

    let _ = try_collect_send(
        "Exchange Orgainization Config",
        run_powershell_get_organization_config(&secrets),
        &splunk,
    )
    .await;

    let _ = try_collect_send(
        "Exchange Sharing Policy",
        run_powershell_get_sharing_policy(&secrets),
        &splunk,
    )
    .await;

    let _ = try_collect_send(
        "Exchange Safe Links Policy",
        run_powershell_get_safe_links_policy(&secrets),
        &splunk,
    )
    .await;

    let _ = try_collect_send(
        "Exchange Malware Filter Policy",
        run_powershell_get_malware_filter_policy(&secrets),
        &splunk,
    )
    .await;

    let _ = try_collect_send(
        "Exchange Hosted Outbound Spam Filter Policy",
        run_powershell_get_hosted_outbound_spam_filter_policy(&secrets),
        &splunk,
    )
    .await;

    let _ = try_collect_send(
        "Exchange Hosted Content Filter Policy",
        run_powershell_get_hosted_content_filter_policy(&secrets),
        &splunk,
    )
    .await;

    let _ = try_collect_send(
        "Exchange Anti Phish Policy",
        run_powershell_get_anti_phish_policy(&secrets),
        &splunk,
    )
    .await;

    let _ = try_collect_send(
        "Exchange Admin Audit Log Config",
        run_powershell_get_admin_audit_log_config(&secrets),
        &splunk,
    )
    .await;

    let _ = try_collect_send(
        "Exchange OWA Mailbox Policy",
        run_powershell_get_owa_mailbox_policy(&secrets),
        &splunk,
    )
    .await;

    let _ = try_collect_send(
        "Exchange Safe Attachment Policy",
        run_powershell_get_safe_attachment_policy(&secrets),
        &splunk,
    )
    .await;

    let _ = try_collect_send(
        "Exchange ATP Polciy for O365",
        run_powershell_get_atp_policy_for_o365(&secrets),
        &splunk,
    )
    .await;

    let _ = try_collect_send(
        "Exchange DLP Complaince Policy",
        run_powershell_get_dlp_compliance_policy(&secrets),
        &splunk,
    )
    .await;

    let _ = try_collect_send(
        "Exchange Transport Rule",
        run_powershell_get_transport_rule(&secrets),
        &splunk,
    )
    .await;

    let _ = try_collect_send(
        "Exchange Dkim Signing Config",
        run_powershell_get_dkim_signing_config(&secrets),
        &splunk,
    )
    .await;

    let _ = try_collect_send(
        "Exchange Spoof Intelligence Insight",
        run_powershell_get_spoof_intelligence_insight(&secrets),
        &splunk,
    )
    .await;

    let _ = try_collect_send(
        "Exchange Blocked Sender Address",
        run_powershell_get_blocked_sender_address(&secrets),
        &splunk,
    )
    .await;

    info!("M365 Powershell Collection Complete");

    Ok(())
}
