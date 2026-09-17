use anyhow::{Context, Result};
use std::env;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};
use tracing::{error, info};

use data_ingester_splunk::splunk::set_ssphp_run;

use data_ingester_splunk::splunk::{try_collect_send, Splunk, SplunkTrait};
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
use crate::powershell::{mailbox_stream_command, parse_mailbox_json_line};

const DEFAULT_MAILBOX_BATCH_SIZE: usize = 100;
const MAILBOX_SOURCE: &str = "powershell:ExchangeOnline:Get-Mailbox";
const MAILBOX_SOURCETYPE: &str = "m365:mailbox";

fn mailbox_batch_size() -> Result<usize> {
    let value =
        env::var("MAILBOX_BATCH_SIZE").unwrap_or_else(|_| DEFAULT_MAILBOX_BATCH_SIZE.to_string());
    let size = value
        .parse::<usize>()
        .with_context(|| format!("Invalid MAILBOX_BATCH_SIZE value: {value}"))?;
    if size == 0 {
        anyhow::bail!("MAILBOX_BATCH_SIZE must be greater than zero");
    }
    Ok(size)
}

async fn collect_mailboxes(secrets: &Secrets, splunk: &Splunk) -> Result<()> {
    let batch_size = mailbox_batch_size()?;
    let command = format!(
        r#" [Byte[]]$pfxBytes = [Convert]::FromBase64String('{}');
$pfx = New-Object System.Security.Cryptography.X509Certificates.X509Certificate2 -ArgumentList (,$pfxBytes);
{}"#,
        secrets
            .azure_client_certificate
            .as_ref()
            .context("Expect azure_client_certificate secret")?,
        mailbox_stream_command(secrets)?,
    );
    let mut child = tokio::process::Command::new("pwsh")
        .args(["-Command", &command])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("Starting PowerShell mailbox collector")?;
    let stdout = child
        .stdout
        .take()
        .context("PowerShell stdout unavailable")?;
    let stderr = child
        .stderr
        .take()
        .context("PowerShell stderr unavailable")?;
    let stderr_task = tokio::spawn(async move {
        let mut bytes = Vec::new();
        let _bytes_read = BufReader::new(stderr).read_to_end(&mut bytes).await?;
        Ok::<_, std::io::Error>(bytes)
    });

    let mut lines = BufReader::new(stdout).lines();
    let mut batch = Vec::with_capacity(batch_size);
    let mut mailbox_count = 0usize;
    let mut malformed_count = 0usize;
    let ssphp_run = data_ingester_splunk::splunk::get_ssphp_run(crate::SSPHP_RUN_KEY);

    while let Some(line) = lines.next_line().await? {
        if line.trim().is_empty() {
            continue;
        }
        let mailbox = match parse_mailbox_json_line(&line) {
            Ok(mailbox) => mailbox,
            Err(err) => {
                malformed_count += 1;
                error!(line = mailbox_count + malformed_count, error = ?err, "Skipping malformed mailbox JSON line");
                continue;
            }
        };
        batch.push(data_ingester_splunk::splunk::HecEvent::new_with_ssphp_run(
            &mailbox,
            MAILBOX_SOURCE,
            MAILBOX_SOURCETYPE,
            ssphp_run,
        )?);
        mailbox_count += 1;
        if batch.len() == batch_size {
            splunk.send_batch(std::mem::take(&mut batch)).await?;
        }
    }

    if !batch.is_empty() {
        splunk.send_batch(batch).await?;
    }
    let status = child.wait().await?;
    let stderr = stderr_task.await??;
    if !status.success() {
        anyhow::bail!(
            "PowerShell mailbox collector failed with {status}: {}",
            String::from_utf8_lossy(&stderr)
        );
    }
    info!(
        mailbox_count,
        malformed_count, "Mailbox collection complete"
    );
    Ok(())
}

pub async fn powershell(secrets: Arc<Secrets>, splunk: Arc<Splunk>) -> Result<()> {
    let _ = set_ssphp_run(crate::SSPHP_RUN_KEY)?;

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
    if let Err(err) = collect_mailboxes(&secrets, &splunk).await {
        error!(error = ?err, "Exchange mailbox collection failed");
    }

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
        "Exchange Organization Config",
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
        "Exchange ATP Policy for O365",
        run_powershell_get_atp_policy_for_o365(&secrets),
        &splunk,
    )
    .await;

    let _ = try_collect_send(
        "Exchange DLP Compliance Policy",
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
