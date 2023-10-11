use anyhow::Result;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
//use tokio::process::Command;
use std::process::Command;

use crate::{keyvault::Secrets, splunk::ToHecEvent};

pub async fn install_powershell() -> Result<()> {
    eprintln!("Downloading Powershell .deb");
    let output = Command::new("curl")
        .args(
            [
                "-L",
                "https://github.com/PowerShell/PowerShell/releases/download/v7.3.7/powershell_7.3.7-1.deb_amd64.deb",
                "-o",
                "/tmp/powershell_7.3.7-1.deb_amd64.deb",
            ]
        )
        .output()?;
    dbg!(output);

    eprintln!("Installing Powershelll .deb");
    let output = Command::new("dpkg")
        .args(["-i", "/tmp/powershell_7.3.7-1.deb_amd64.deb"])
        .output()?;
    dbg!(output);

    eprintln!("Installing Powershelll ExchangeOnlineManagement");
    let output = Command::new("pwsh")
        .args([
            "-Command",
            r#"
Install-Module -Confirm:$False -Force -Name ExchangeOnlineManagement;
"#,
        ])
        .output()?;
    dbg!(output);

    //     eprintln!("Installing Powershelll PSWSMan");
    //     let output = Command::new("pwsh")
    //         .args([
    //             "-Command",
    //             r#"
    // Install-Module -Confirm:$False -Force -Name PSWSMan;
    // "#,
    //         ])
    //         .output()?;
    //     dbg!(output);

    //     eprintln!("Installing Powershelll Microsoft.Graph");
    //     let output = Command::new("pwsh")
    //         .args([
    //             "-Command",
    //             r#"
    // Install-Module -Confirm:$False -Force -AllowClobber -Name Microsoft.Graph;
    // "#,
    //         ])
    //         .output()?;
    //     dbg!(output);

    //     eprintln!("Installing Powershelll Microsoft.Graph.Beta");
    //     let output = Command::new("pwsh")
    //         .args([
    //             "-Command",
    //             r#"
    // Install-Module -Confirm:$False -Force -AllowClobber -Name Microsoft.Graph.Beta;
    // "#,
    //         ])
    //         .output()?;
    //     dbg!(output);

    Ok(())
}

pub async fn run_powershell_get_organization_config(
    secrets: &Secrets,
) -> Result<ExchangeOrganizationConfig> {
    let command = "Get-OrganizationConfig";
    let result = run_exchange_online_powershell(secrets, command).await?;
    Ok(result)
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExchangeOrganizationConfig(serde_json::Value);

impl ToHecEvent for ExchangeOrganizationConfig {
    fn source() -> &'static str {
        "powershell:ExchangeOnline:Get-OrganizationConfig"
    }

    fn sourcetype() -> &'static str {
        "m365:organization_config"
    }
}

pub async fn run_powershell_get_safe_links_policy(secrets: &Secrets) -> Result<SafeLinksPolicy> {
    let command = "Get-SafeLinksPolicy";
    let result = run_exchange_online_powershell(secrets, command).await?;
    Ok(result)
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SafeLinksPolicy(serde_json::Value);

impl ToHecEvent for SafeLinksPolicy {
    fn source() -> &'static str {
        "powershell:ExchangeOnline:Get-SafeLinksPolicy"
    }

    fn sourcetype() -> &'static str {
        "m365:safe_links_policy"
    }
}

pub async fn run_powershell_get_sharing_policy(secrets: &Secrets) -> Result<SharingPolicy> {
    let command = "Get-SharingPolicy";
    let result = run_exchange_online_powershell(secrets, command).await?;
    Ok(result)
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SharingPolicy(serde_json::Value);

impl ToHecEvent for SharingPolicy {
    fn source() -> &'static str {
        "powershell:ExchangeOnline:Get-SharingPolicy"
    }

    fn sourcetype() -> &'static str {
        "m365:sharing_policy"
    }
}

pub async fn run_powershell_get_malware_filter_policy(
    secrets: &Secrets,
) -> Result<MalwareFilterPolicy> {
    let command = "Get-MalwareFilterPolicy";
    let result = run_exchange_online_powershell(secrets, command).await?;
    Ok(result)
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MalwareFilterPolicy(serde_json::Value);

impl ToHecEvent for MalwareFilterPolicy {
    fn source() -> &'static str {
        "powershell:ExchangeOnline:Get-MalwareFilterPolicy"
    }

    fn sourcetype() -> &'static str {
        "m365:malware_filter_policy"
    }
}

pub async fn run_powershell_get_hosted_outbound_spam_filter_policy(
    secrets: &Secrets,
) -> Result<HostedOutboundSpamFilterPolicy> {
    let command = "Get-HostedOutboundSpamFilterPolicy";
    let result = run_exchange_online_powershell(secrets, command).await?;
    Ok(result)
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HostedOutboundSpamFilterPolicy(serde_json::Value);

impl ToHecEvent for HostedOutboundSpamFilterPolicy {
    fn source() -> &'static str {
        "powershell:ExchangeOnline:Get-HostedOutboundSpamFilterPolicy"
    }

    fn sourcetype() -> &'static str {
        "m365:hosted_outbound_spam_filter_policy"
    }
}

pub async fn run_powershell_get_anti_phish_policy(secrets: &Secrets) -> Result<AntiPhishPolicy> {
    let command = "Get-AntiPhishPolicy";
    let result = run_exchange_online_powershell(secrets, command).await?;
    Ok(result)
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AntiPhishPolicy(serde_json::Value);

impl ToHecEvent for AntiPhishPolicy {
    fn source() -> &'static str {
        "powershell:ExchangeOnline:Get-AntiPhishPolicy"
    }

    fn sourcetype() -> &'static str {
        "m365:anti_phish_policy"
    }
}

pub async fn run_powershell_get_admin_audit_log_config(
    secrets: &Secrets,
) -> Result<AdminAuditLogConfig> {
    let command = "Get-AdminAuditLogConfig";
    let result = run_exchange_online_powershell(secrets, command).await?;
    Ok(result)
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminAuditLogConfig(serde_json::Value);

impl ToHecEvent for AdminAuditLogConfig {
    fn source() -> &'static str {
        "powershell:ExchangeOnline:Get-AdminAuditLogConfig"
    }

    fn sourcetype() -> &'static str {
        "m365:admin_audit_log_config"
    }
}

pub async fn run_powershell_get_owa_mailbox_policy(secrets: &Secrets) -> Result<OwaMailboxPolicy> {
    let command = "Get-OwaMailboxPolicy";
    let result = run_exchange_online_powershell(secrets, command).await?;
    Ok(result)
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OwaMailboxPolicy(serde_json::Value);

impl ToHecEvent for OwaMailboxPolicy {
    fn source() -> &'static str {
        "powershell:ExchangeOnline:Get-OwaMailboxPolicy"
    }

    fn sourcetype() -> &'static str {
        "m365:owa_mailbox_policy"
    }
}

pub async fn run_exchange_online_powershell<T: DeserializeOwned>(
    secrets: &Secrets,
    command: &str,
) -> Result<T> {
    let output = Command::new("pwsh")
        .args([
            "-Command",
            &format!(r#"
[Byte[]]$pfxBytes = [Convert]::FromBase64String('{}');
$pfx = New-Object System.Security.Cryptography.X509Certificates.X509Certificate -ArgumentList (,$pfxBytes);
Import-Module ExchangeOnlineManagement;
Connect-ExchangeOnline -ShowBanner:$false -Certificate $pfx -AppID "{}" -Organization "{}";
{} | ConvertTo-Json -Compress;"#,
                     secrets.azure_client_certificate,
                     secrets.azure_client_id,
                     secrets.azure_client_organization,
                     command,
            )
        ]).output()?;
    // dbg!(&output);
    let out = serde_json::from_slice::<T>(&output.stdout[..])?;
    Ok(out)
}

#[cfg(test)]
mod test {
    use crate::{
        keyvault::{get_keyvault_secrets, Secrets},
        powershell::{
            install_powershell, run_powershell_get_admin_audit_log_config,
            run_powershell_get_anti_phish_policy,
            run_powershell_get_hosted_outbound_spam_filter_policy,
            run_powershell_get_malware_filter_policy, run_powershell_get_organization_config,
            run_powershell_get_owa_mailbox_policy, run_powershell_get_safe_links_policy,
            run_powershell_get_sharing_policy,
        },
        splunk::{set_ssphp_run, Splunk, ToHecEvent},
    };
    use anyhow::Result;
    use std::env;

    async fn setup() -> Result<(Splunk, Secrets)> {
        let secrets = get_keyvault_secrets(&env::var("KEY_VAULT_NAME")?).await?;

        set_ssphp_run()?;

        let splunk = Splunk::new(&secrets.splunk_host, &secrets.splunk_token)?;
        Ok((splunk, secrets))
    }

    #[ignore]
    #[tokio::test]
    async fn test_install_powershell() -> Result<()> {
        install_powershell().await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_run_powershell_get_organization_config() -> Result<()> {
        let (splunk, secrets) = setup().await?;
        let exchange_org_config = run_powershell_get_organization_config(&secrets).await?;
        splunk
            .send_batch(&[exchange_org_config.to_hec_event()?])
            .await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_run_powershell_get_sharing_policy() -> Result<()> {
        let (splunk, secrets) = setup().await?;
        let sharing_policy = run_powershell_get_sharing_policy(&secrets).await?;
        splunk.send_batch(&[sharing_policy.to_hec_event()?]).await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_run_powershell_get_safe_links_policy() -> Result<()> {
        let (splunk, secrets) = setup().await?;
        let result = run_powershell_get_safe_links_policy(&secrets).await?;
        splunk.send_batch(&[result.to_hec_event()?]).await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_run_powershell_get_malware_filter_policy() -> Result<()> {
        let (splunk, secrets) = setup().await?;
        let result = run_powershell_get_malware_filter_policy(&secrets).await?;
        splunk.send_batch(&[result.to_hec_event()?]).await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_run_powershell_get_hosted_outbound_spam_filter_policy() -> Result<()> {
        let (splunk, secrets) = setup().await?;
        let result = run_powershell_get_hosted_outbound_spam_filter_policy(&secrets).await?;
        splunk.send_batch(&[result.to_hec_event()?]).await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_run_powershell_get_anti_phish_policy() -> Result<()> {
        let (splunk, secrets) = setup().await?;
        let result = run_powershell_get_anti_phish_policy(&secrets).await?;
        splunk.send_batch(&[result.to_hec_event()?]).await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_run_powershell_get_admin_audit_log_config() -> Result<()> {
        let (splunk, secrets) = setup().await?;
        let result = run_powershell_get_admin_audit_log_config(&secrets).await?;
        splunk.send_batch(&[result.to_hec_event()?]).await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_run_powershell_get_owa_mailbox_policy() -> Result<()> {
        let (splunk, secrets) = setup().await?;
        let result = run_powershell_get_owa_mailbox_policy(&secrets).await?;
        splunk.send_batch(&[result.to_hec_event()?]).await?;
        Ok(())
    }
}
