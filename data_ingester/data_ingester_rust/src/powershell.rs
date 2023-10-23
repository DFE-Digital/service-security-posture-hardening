use anyhow::Result;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::iter;
use std::process::Command;

use crate::{keyvault::Secrets, splunk::ToHecEvents};

pub async fn install_powershell() -> Result<()> {
    eprintln!("Downloading Powershell .deb");
    let _output = Command::new("curl")
        .args(
            [
                "-L",
                "https://github.com/PowerShell/PowerShell/releases/download/v7.3.7/powershell_7.3.7-1.deb_amd64.deb",
                "-o",
                "/tmp/powershell_7.3.7-1.deb_amd64.deb",
            ]
        )
        .output()?;

    eprintln!("Installing Powershelll .deb");
    let _output = Command::new("dpkg")
        .args(["-i", "/tmp/powershell_7.3.7-1.deb_amd64.deb"])
        .output()?;

    eprintln!("Installing Powershelll ExchangeOnlineManagement");
    let _output = Command::new("pwsh")
        .args([
            "-Command",
            r#"
Install-Module -Confirm:$False -Force -Name ExchangeOnlineManagement;
"#,
        ])
        .output()?;

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

impl ToHecEvents for &ExchangeOrganizationConfig {
    type Item = Self;
    fn source(&self) -> &str {
        "powershell:ExchangeOnline:Get-OrganizationConfig"
    }

    fn sourcetype(&self) -> &str {
        "m365:organization_config"
    }

    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        Box::new(iter::once(self))
    }
}

pub async fn run_powershell_get_safe_links_policy(secrets: &Secrets) -> Result<SafeLinksPolicy> {
    let command = "Get-SafeLinksPolicy";
    let result = run_exchange_online_powershell(secrets, command).await?;
    Ok(result)
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SafeLinksPolicy(Vec<serde_json::Value>);

impl ToHecEvents for &SafeLinksPolicy {
    type Item = serde_json::Value;
    fn source(&self) -> &'static str {
        "powershell:ExchangeOnline:Get-SafeLinksPolicy"
    }

    fn sourcetype(&self) -> &'static str {
        "m365:safe_links_policy"
    }

    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        Box::new(self.0.iter())
    }
}

pub async fn run_powershell_get_sharing_policy(secrets: &Secrets) -> Result<SharingPolicy> {
    let command = "Get-SharingPolicy";
    let result = run_exchange_online_powershell(secrets, command).await?;
    Ok(result)
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(untagged)]
pub enum SharingPolicy {
    Collection(Vec<serde_json::Value>),
    Single(serde_json::Value),
}

impl ToHecEvents for &SharingPolicy {
    type Item = serde_json::Value;
    fn source(&self) -> &'static str {
        "powershell:ExchangeOnline:Get-SharingPolicy"
    }

    fn sourcetype(&self) -> &'static str {
        "m365:sharing_policy"
    }

    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        match self {
            SharingPolicy::Collection(collection) => Box::new(collection.iter()),
            SharingPolicy::Single(single) => Box::new(iter::once(single)),
        }
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

impl ToHecEvents for &MalwareFilterPolicy {
    type Item = Self;
    fn source(&self) -> &'static str {
        "powershell:ExchangeOnline:Get-MalwareFilterPolicy"
    }

    fn sourcetype(&self) -> &'static str {
        "m365:malware_filter_policy"
    }

    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        Box::new(iter::once(self))
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

impl ToHecEvents for &HostedOutboundSpamFilterPolicy {
    type Item = Self;
    fn source(&self) -> &str {
        "powershell:ExchangeOnline:Get-HostedOutboundSpamFilterPolicy"
    }

    fn sourcetype(&self) -> &str {
        "m365:hosted_outbound_spam_filter_policy"
    }

    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        Box::new(iter::once(self))
    }
}

pub async fn run_powershell_get_anti_phish_policy(secrets: &Secrets) -> Result<AntiPhishPolicy> {
    let command = "Get-AntiPhishPolicy";
    let result = run_exchange_online_powershell(secrets, command).await?;
    Ok(result)
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(untagged)]
pub enum AntiPhishPolicy {
    Collection(Vec<serde_json::Value>),
    Single(serde_json::Value),
}

impl ToHecEvents for &AntiPhishPolicy {
    type Item = serde_json::Value;
    fn source(&self) -> &str {
        "powershell:ExchangeOnline:Get-AntiPhishPolicy"
    }

    fn sourcetype(&self) -> &str {
        "m365:anti_phish_policy"
    }

    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        match self {
            AntiPhishPolicy::Collection(collection) => Box::new(collection.iter()),
            AntiPhishPolicy::Single(single) => Box::new(iter::once(single)),
        }
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

impl ToHecEvents for &AdminAuditLogConfig {
    type Item = Self;
    fn source(&self) -> &str {
        "powershell:ExchangeOnline:Get-AdminAuditLogConfig"
    }

    fn sourcetype(&self) -> &str {
        "m365:admin_audit_log_config"
    }

    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        Box::new(iter::once(self))
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

impl ToHecEvents for &OwaMailboxPolicy {
    type Item = Self;
    fn source(&self) -> &'static str {
        "powershell:ExchangeOnline:Get-OwaMailboxPolicy"
    }

    fn sourcetype(&self) -> &'static str {
        "m365:owa_mailbox_policy"
    }

    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        Box::new(iter::once(self))
    }
}

pub async fn run_powershell_get_safe_attachment_policy(
    secrets: &Secrets,
) -> Result<SafeAttachmentPolicy> {
    let command = "Get-SafeAttachmentPolicy";
    let result = run_exchange_online_powershell(secrets, command).await?;
    Ok(result)
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SafeAttachmentPolicy(Vec<serde_json::Value>);

impl ToHecEvents for &SafeAttachmentPolicy {
    type Item = serde_json::Value;
    fn source(&self) -> &'static str {
        "powershell:ExchangeOnline:Get-SafeAttachmentPolicy"
    }

    fn sourcetype(&self) -> &'static str {
        "m365:safe_attachment_policy"
    }

    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        Box::new(self.0.iter())
    }
}

pub async fn run_powershell_get_atp_policy_for_o365(secrets: &Secrets) -> Result<AtpPolciyForO365> {
    let command = "Get-AtpPolicyForO365";
    let result = run_exchange_online_powershell(secrets, command).await?;
    Ok(result)
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AtpPolciyForO365(serde_json::Value);

impl ToHecEvents for &AtpPolciyForO365 {
    type Item = Self;
    fn source(&self) -> &'static str {
        "powershell:ExchangeOnline:Get-AtpPolicyForO365"
    }

    fn sourcetype(&self) -> &'static str {
        "m365:atp_policy_for_o365"
    }

    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        Box::new(iter::once(self))
    }
}

pub async fn run_powershell_get_dlp_compliance_policy(
    secrets: &Secrets,
) -> Result<DlpCompliancePolicy> {
    let command = "Get-DlpCompliancePolicy";
    let result = run_exchange_online_ipps_powershell(secrets, command).await?;
    Ok(result)
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DlpCompliancePolicy(serde_json::Value);

impl ToHecEvents for &DlpCompliancePolicy {
    type Item = Self;
    fn source(&self) -> &'static str {
        "powershell:ExchangeOnline:Get-DlpCompliancePolicy"
    }

    fn sourcetype(&self) -> &'static str {
        "m365:dlp_compliance_policy"
    }

    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        Box::new(iter::once(self))
    }
}

pub async fn run_powershell_get_transport_rule(secrets: &Secrets) -> Result<TransportRule> {
    let command = "Get-TransportRule";
    let result = run_exchange_online_powershell(secrets, command).await?;
    Ok(result)
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransportRule(Vec<serde_json::Value>);

impl ToHecEvents for &TransportRule {
    type Item = serde_json::Value;
    fn source(&self) -> &'static str {
        "powershell:ExchangeOnline:Get-TransportRule"
    }

    fn sourcetype(&self) -> &'static str {
        "m365:transport_rule"
    }

    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        Box::new(self.0.iter())
    }
}

pub async fn run_powershell_get_dkim_signing_config(secrets: &Secrets) -> Result<TransportRule> {
    let command = "Get-DkimSigningConfig";
    let result = run_exchange_online_powershell(secrets, command).await?;
    Ok(result)
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DkimSigningConfig(Vec<serde_json::Value>);

impl ToHecEvents for &DkimSigningConfig {
    type Item = serde_json::Value;
    fn source(&self) -> &'static str {
        "powershell:ExchangeOnline:Get-DkimSigningConfig"
    }

    fn sourcetype(&self) -> &'static str {
        "m365:dkim_signing_config"
    }

    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        Box::new(self.0.iter())
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

    let out = serde_json::from_slice::<T>(&output.stdout[..])?;
    Ok(out)
}

pub async fn run_exchange_online_ipps_powershell<T: DeserializeOwned>(
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
Connect-IPPSSession -ShowBanner:$false -Certificate $pfx -AppID "{}" -Organization "{}";
{} | ConvertTo-Json -Compress;"#,
                     secrets.azure_client_certificate,
                     secrets.azure_client_id,
                     secrets.azure_client_organization,
                     command,
            )
        ]).output()?;

    let out = serde_json::from_slice::<T>(&output.stdout[..])?;
    Ok(out)
}

#[cfg(test)]
mod test {
    use crate::{
        keyvault::{get_keyvault_secrets, Secrets},
        powershell::{
            install_powershell, run_powershell_get_admin_audit_log_config,
            run_powershell_get_anti_phish_policy, run_powershell_get_atp_policy_for_o365,
            run_powershell_get_dlp_compliance_policy,
            run_powershell_get_hosted_outbound_spam_filter_policy,
            run_powershell_get_malware_filter_policy, run_powershell_get_organization_config,
            run_powershell_get_owa_mailbox_policy, run_powershell_get_safe_attachment_policy,
            run_powershell_get_safe_links_policy, run_powershell_get_sharing_policy,
            run_powershell_get_transport_rule,
        },
        splunk::{set_ssphp_run, Splunk, ToHecEvents},
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
            .send_batch((&exchange_org_config).to_hec_events()?)
            .await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_run_powershell_get_sharing_policy() -> Result<()> {
        let (splunk, secrets) = setup().await?;
        let sharing_policy = run_powershell_get_sharing_policy(&secrets).await?;
        splunk
            .send_batch((&sharing_policy).to_hec_events()?)
            .await?;
        Ok(())
    }

    #[ignore]
    #[tokio::test]
    async fn test_run_powershell_get_safe_links_policy() -> Result<()> {
        let (splunk, secrets) = setup().await?;
        let result = run_powershell_get_safe_links_policy(&secrets).await?;
        splunk.send_batch((&result).to_hec_events()?).await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_run_powershell_get_malware_filter_policy() -> Result<()> {
        let (splunk, secrets) = setup().await?;
        let result = run_powershell_get_malware_filter_policy(&secrets).await?;
        splunk.send_batch((&result).to_hec_events()?).await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_run_powershell_get_hosted_outbound_spam_filter_policy() -> Result<()> {
        let (splunk, secrets) = setup().await?;
        let result = run_powershell_get_hosted_outbound_spam_filter_policy(&secrets).await?;
        splunk.send_batch((&result).to_hec_events()?).await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_run_powershell_get_anti_phish_policy() -> Result<()> {
        let (splunk, secrets) = setup().await?;
        let result = run_powershell_get_anti_phish_policy(&secrets).await?;
        splunk.send_batch((&result).to_hec_events()?).await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_run_powershell_get_admin_audit_log_config() -> Result<()> {
        let (splunk, secrets) = setup().await?;
        let result = run_powershell_get_admin_audit_log_config(&secrets).await?;
        splunk.send_batch((&result).to_hec_events()?).await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_run_powershell_get_owa_mailbox_policy() -> Result<()> {
        let (splunk, secrets) = setup().await?;
        let result = run_powershell_get_owa_mailbox_policy(&secrets).await?;
        splunk.send_batch((&result).to_hec_events()?).await?;
        Ok(())
    }

    #[ignore]
    #[tokio::test]
    async fn test_run_powershell_get_safe_attachment_policy() -> Result<()> {
        let (splunk, secrets) = setup().await?;
        let result = run_powershell_get_safe_attachment_policy(&secrets).await?;
        splunk.send_batch((&result).to_hec_events()?).await?;
        Ok(())
    }

    #[ignore]
    #[tokio::test]
    async fn test_run_powershell_get_atp_policy_for_o365() -> Result<()> {
        let (splunk, secrets) = setup().await?;
        let result = run_powershell_get_atp_policy_for_o365(&secrets).await?;
        splunk.send_batch((&result).to_hec_events()?).await?;
        Ok(())
    }

    #[ignore]
    #[tokio::test]
    async fn test_run_powershell_get_dlp_compliance_policy() -> Result<()> {
        let (splunk, secrets) = setup().await?;
        let result = run_powershell_get_dlp_compliance_policy(&secrets).await?;
        splunk.send_batch((&result).to_hec_events()?).await?;
        Ok(())
    }

    #[ignore]
    #[tokio::test]
    async fn test_run_powershell_get_transport_rule() -> Result<()> {
        let (splunk, secrets) = setup().await?;
        let result = run_powershell_get_transport_rule(&secrets).await?;
        splunk.send_batch((&result).to_hec_events()?).await?;
        Ok(())
    }

    #[ignore]
    #[tokio::test]
    async fn test_run_powershell_get_dkim_signing_config() -> Result<()> {
        let (splunk, secrets) = setup().await?;
        let result = run_powershell_get_transport_rule(&secrets).await?;
        splunk.send_batch((&result).to_hec_events()?).await?;
        Ok(())
    }
}