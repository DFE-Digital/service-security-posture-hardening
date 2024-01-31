use anyhow::Result;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::Value;
use std::iter;
use std::process::Command;

use data_ingester_supporting::keyvault::Secrets;
use data_ingester_splunk::splunk::ToHecEvents;

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

    eprintln!("Installing Powershelll MicrosoftTeams");
    let _output = Command::new("pwsh")
        .args([
            "-Command",
            r#"
Install-Module -Confirm:$False -Force -Name MicrosoftTeams;
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(untagged)]
pub enum MalwareFilterPolicy {
    Collection(Vec<serde_json::Value>),
    Single(serde_json::Value),
}

impl ToHecEvents for &MalwareFilterPolicy {
    type Item = Value;
    fn source(&self) -> &'static str {
        "powershell:ExchangeOnline:Get-MalwareFilterPolicy"
    }

    fn sourcetype(&self) -> &'static str {
        "m365:malware_filter_policy"
    }

    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        match self {
            MalwareFilterPolicy::Collection(collection) => Box::new(collection.iter()),
            MalwareFilterPolicy::Single(single) => Box::new(iter::once(single)),
        }
    }
}

pub async fn run_powershell_get_eop_protection_policy_rule(
    secrets: &Secrets,
) -> Result<EopProtectionPolicyRule> {
    let command = "Get-EOPProtectionPolicyRule";
    let result = run_exchange_online_powershell(secrets, command).await?;
    Ok(result)
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(untagged)]
pub enum EopProtectionPolicyRule {
    Collection(Vec<serde_json::Value>),
    Single(serde_json::Value),
}

impl ToHecEvents for &EopProtectionPolicyRule {
    type Item = Value;
    fn source(&self) -> &'static str {
        "powershell:ExchangeOnline:Get-EOPProtectionPolicyRule"
    }

    fn sourcetype(&self) -> &'static str {
        "m365:eop_protection_policy_rule"
    }

    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        match self {
            EopProtectionPolicyRule::Collection(collection) => Box::new(collection.iter()),
            EopProtectionPolicyRule::Single(single) => Box::new(iter::once(single)),
        }
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

/// TODO Casuses deserialization errors. Need to limit the fields to the ones used.
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

pub async fn run_powershell_get_mailbox(secrets: &Secrets) -> Result<Mailboxes> {
    let command = "Get-Mailbox -ResultSize Unlimited";
    let result = run_exchange_online_powershell(secrets, command).await?;
    Ok(result)
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Mailboxes(Vec<Mailbox>);

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Mailbox {
    id: String,
    audit_enabled: bool,
    recipient_type_details: String,
}

impl ToHecEvents for &Mailboxes {
    type Item = Mailbox;
    fn source(&self) -> &'static str {
        "powershell:ExchangeOnline:Get-Mailbox"
    }

    fn sourcetype(&self) -> &'static str {
        "m365:mailbox"
    }

    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        Box::new(self.0.iter())
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(untagged)]
pub enum DlpCompliancePolicy {
    Collection(Vec<Value>),
    Single(Value),
}

impl ToHecEvents for &DlpCompliancePolicy {
    type Item = Value;
    fn source(&self) -> &'static str {
        "powershell:ExchangeOnline:Get-DlpCompliancePolicy"
    }

    fn sourcetype(&self) -> &'static str {
        "m365:dlp_compliance_policy"
    }

    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        match self {
            DlpCompliancePolicy::Collection(collection) => Box::new(collection.iter()),
            DlpCompliancePolicy::Single(single) => Box::new(iter::once(single)),
        }
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

pub async fn run_powershell_get_dkim_signing_config(
    secrets: &Secrets,
) -> Result<DkimSigningConfig> {
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

pub async fn run_powershell_get_spoof_intelligence_insight(
    secrets: &Secrets,
) -> Result<SpoofIntelligenceInsight> {
    let command = "Get-SpoofIntelligenceInsight";
    let result = run_exchange_online_powershell(secrets, command).await?;
    Ok(result)
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpoofIntelligenceInsight(Vec<serde_json::Value>);

impl ToHecEvents for &SpoofIntelligenceInsight {
    type Item = serde_json::Value;
    fn source(&self) -> &'static str {
        "powershell:ExchangeOnline:Get-SpoofIntelligenceInsight"
    }

    fn sourcetype(&self) -> &'static str {
        "m365:spoof_intelligence_insight"
    }

    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        Box::new(self.0.iter())
    }
}

pub async fn run_powershell_get_blocked_sender_address(
    secrets: &Secrets,
) -> Result<BlockedSenderAddress> {
    let command = "Get-BlockedSenderAddress";
    let result = run_exchange_online_powershell(secrets, command).await?;
    Ok(result)
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockedSenderAddress(serde_json::Value);

impl ToHecEvents for &BlockedSenderAddress {
    type Item = Self;
    fn source(&self) -> &'static str {
        "powershell:ExchangeOnline:Get-BlockedSenderAddress"
    }

    fn sourcetype(&self) -> &'static str {
        "m365:blocked_sender_address"
    }

    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        Box::new(iter::once(self))
    }
}

/// 4.12
pub async fn run_powershell_get_email_tenant_settings(
    secrets: &Secrets,
) -> Result<EmailTenantSettings> {
    let command = "Get-EmailTenantSettings";
    let result = run_exchange_online_powershell(secrets, command).await?;
    Ok(result)
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EmailTenantSettings(serde_json::Value);

impl ToHecEvents for &EmailTenantSettings {
    type Item = Self;
    fn source(&self) -> &'static str {
        "powershell:ExchangeOnline:Get-EmailTenantSettings"
    }

    fn sourcetype(&self) -> &'static str {
        "m365:email_tenant_settings"
    }

    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        Box::new(iter::once(self))
    }
}

pub async fn run_powershell_get_user_vip(secrets: &Secrets) -> Result<UserVip> {
    let command = "Get-User -IsVIP";
    let result = run_exchange_online_powershell(secrets, command).await?;
    Ok(result)
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(untagged)]
pub enum UserVip {
    Collection(Vec<UserVipDetails>),
    Single(UserVipDetails),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct UserVipDetails {
    id: String,
    recipient_type: String,
    object_category: String,
    object_class: Vec<String>,
}

impl ToHecEvents for &UserVip {
    type Item = UserVipDetails;
    // type Item = Value;
    fn source(&self) -> &'static str {
        "powershell:ExchangeOnline:Get-User-Is-Vip"
    }

    fn sourcetype(&self) -> &'static str {
        "m365:user_vip"
    }

    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        match self {
            UserVip::Collection(collection) => Box::new(collection.iter()),
            UserVip::Single(single) => Box::new(iter::once(single)),
        }
    }
}

// 4.12
pub async fn run_powershell_get_protection_alert(secrets: &Secrets) -> Result<ProtectionAlert> {
    let command = "Get-ProtectionAlert";
    let result = run_exchange_online_ipps_powershell(secrets, command).await?;
    Ok(result)
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProtectionAlert(Vec<serde_json::Value>);

impl ToHecEvents for &ProtectionAlert {
    type Item = Value;
    fn source(&self) -> &'static str {
        "powershell:ExchangeOnline:Get-ProtectionAlert"
    }

    fn sourcetype(&self) -> &'static str {
        "m365:protection_alert"
    }

    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        Box::new(self.0.iter())
    }
}

// 4.13
pub async fn run_powershell_get_hosted_content_filter_policy(
    secrets: &Secrets,
) -> Result<HostedContentFilterPolicy> {
    let command = "Get-HostedContentFilterPolicy";
    let result = run_exchange_online_powershell(secrets, command).await?;
    Ok(result)
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(untagged)]
pub enum HostedContentFilterPolicy {
    Collection(Vec<Value>),
    Single(Value),
}

impl ToHecEvents for &HostedContentFilterPolicy {
    type Item = Value;
    fn source(&self) -> &'static str {
        "powershell:ExchangeOnline:Get-HostedContentFilterPolicy"
    }

    fn sourcetype(&self) -> &'static str {
        "m365:hosted_content_filter_policy"
    }

    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        match self {
            HostedContentFilterPolicy::Collection(collection) => Box::new(collection.iter()),
            HostedContentFilterPolicy::Single(single) => Box::new(iter::once(single)),
        }
    }
}

// 3.7
pub async fn run_powershell_get_cs_teams_client_configuration(
    secrets: &Secrets,
) -> Result<CsTeamsClientConfiguration> {
    let command = "Get-CsTeamsClientConfiguration";
    let result = run_microsoft_teams_powershell(secrets, command).await?;
    Ok(result)
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(untagged)]
pub enum CsTeamsClientConfiguration {
    Collection(Vec<Value>),
    Single(Value),
}

impl ToHecEvents for &CsTeamsClientConfiguration {
    type Item = Value;
    fn source(&self) -> &'static str {
        "powershell:MicrosoftTeams:Get-CsTeamsClientConfiguration"
    }

    fn sourcetype(&self) -> &'static str {
        "m365:get_cs_teams_client_configuration"
    }

    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        match self {
            CsTeamsClientConfiguration::Collection(collection) => Box::new(collection.iter()),
            CsTeamsClientConfiguration::Single(single) => Box::new(iter::once(single)),
        }
    }
}

// 3.6
pub async fn run_powershell_get_cs_tenant_federation_configuration(
    secrets: &Secrets,
) -> Result<CsTenantFederationConfiguration> {
    let command = "Get-CsTenantFederationConfiguration";
    let result = run_microsoft_teams_powershell(secrets, command).await?;
    Ok(result)
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(untagged)]
pub enum CsTenantFederationConfiguration {
    Collection(Vec<Value>),
    Single(Value),
}

impl ToHecEvents for &CsTenantFederationConfiguration {
    type Item = Value;
    fn source(&self) -> &'static str {
        "powershell:MicrosoftTeams:Get-CsTenantFederationConfiguration"
    }

    fn sourcetype(&self) -> &'static str {
        "m365:get_cs_teams_tenant_federation_configuration"
    }

    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        match self {
            CsTenantFederationConfiguration::Collection(collection) => Box::new(collection.iter()),
            CsTenantFederationConfiguration::Single(single) => Box::new(iter::once(single)),
        }
    }
}

// M365 V2 2.8
pub async fn run_powershell_get_management_role_assignment(
    secrets: &Secrets,
) -> Result<ManagementRoleAssignment> {
    let command = "Get-ManagementRoleAssignment";
    let result = run_exchange_online_powershell(secrets, command).await?;
    Ok(result)
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(untagged)]
pub enum ManagementRoleAssignment {
    Collection(Vec<Value>),
    Single(Value),
}

impl ToHecEvents for &ManagementRoleAssignment {
    type Item = Value;
    fn source(&self) -> &'static str {
        "powershell:MicrosoftTeams:Get-ManagementRoleAssignment"
    }

    fn sourcetype(&self) -> &'static str {
        "m365:get_management_role_assignment"
    }

    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        match self {
            ManagementRoleAssignment::Collection(collection) => Box::new(collection.iter()),
            ManagementRoleAssignment::Single(single) => Box::new(iter::once(single)),
        }
    }
}

pub async fn run_powershell_exchange_login_test(secrets: &Secrets) -> Result<LoginTest> {
    let output = Command::new("pwsh")
        .args([
            "-Command",
            &format!(r#"
[Byte[]]$pfxBytes = [Convert]::FromBase64String('{}');
$pfx = New-Object System.Security.Cryptography.X509Certificates.X509Certificate -ArgumentList (,$pfxBytes);
Import-Module ExchangeOnlineManagement;
Connect-ExchangeOnline -Certificate $pfx -AppID "{}" -Organization "{}";"#,
                     secrets.azure_client_certificate,
                     secrets.azure_client_id,
                     secrets.azure_client_organization,
            )
        ]).output()?;

    let stdout = String::from_utf8(output.stdout)?;
    let stderr = String::from_utf8(output.stderr)?;

    Ok(LoginTest::Single(Output { stdout, stderr }))
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Output {
    stdout: String,
    stderr: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(untagged)]
pub enum LoginTest {
    Collection(Vec<Output>),
    Single(Output),
}

impl ToHecEvents for &LoginTest {
    type Item = Output;
    fn source(&self) -> &'static str {
        "powershell:Exchange:connect_exchange:loging_type"
    }

    fn sourcetype(&self) -> &'static str {
        "m365:Connet-ExchangOnline:login_test"
    }

    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        match self {
            LoginTest::Collection(collection) => Box::new(collection.iter()),
            LoginTest::Single(single) => Box::new(iter::once(single)),
        }
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
{} | ConvertTo-Json -Compress -Depth 100;"#,
                     secrets.azure_client_certificate,
                     secrets.azure_client_id,
                     secrets.azure_client_organization,
                     &command,
            )
        ]).output()?;

    match serde_json::from_slice::<T>(&output.stdout[..]) {
        Ok(out) => Ok(out),
        Err(error) => {
            eprintln!(
                "Error while serializing data from: {}, {}",
                &command, &error
            );
            eprintln!("output: {}", String::from_utf8(output.stdout)?);
            Err(error.into())
        }
    }
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
{} | ConvertTo-Json -Compress -Depth 20;"#,
                     secrets.azure_client_certificate,
                     secrets.azure_client_id,
                     secrets.azure_client_organization,
                     command,
            )
        ]).output()?;

    let out = serde_json::from_slice::<T>(&output.stdout[..])?;
    Ok(out)
}

pub async fn run_microsoft_teams_powershell<T: DeserializeOwned>(
    secrets: &Secrets,
    command: &str,
) -> Result<T> {
    let output = Command::new("pwsh")
        .args([
            "-Command",
            &format!(r#"
[Byte[]]$pfxBytes = [Convert]::FromBase64String('{}');
$pfx = New-Object System.Security.Cryptography.X509Certificates.X509Certificate2 -ArgumentList (,$pfxBytes);
Import-Module MicrosoftTeams;
Connect-MicrosoftTeams -Certificate $pfx -ApplicationId "{}" -TenantId "{}" | Out-Null;
{} | ConvertTo-Json -Compress -Depth 20;"#,
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


    use crate::powershell::{
            install_powershell, run_powershell_exchange_login_test,
            run_powershell_get_admin_audit_log_config, run_powershell_get_anti_phish_policy,
            run_powershell_get_atp_policy_for_o365, run_powershell_get_blocked_sender_address,
            run_powershell_get_cs_teams_client_configuration,
            run_powershell_get_cs_tenant_federation_configuration,
            run_powershell_get_dkim_signing_config, run_powershell_get_dlp_compliance_policy,
            run_powershell_get_email_tenant_settings,
            run_powershell_get_eop_protection_policy_rule,
            run_powershell_get_hosted_content_filter_policy,
            run_powershell_get_hosted_outbound_spam_filter_policy, run_powershell_get_mailbox,
            run_powershell_get_malware_filter_policy,
            run_powershell_get_management_role_assignment, run_powershell_get_organization_config,
            run_powershell_get_owa_mailbox_policy, run_powershell_get_protection_alert,
            run_powershell_get_safe_attachment_policy, run_powershell_get_safe_links_policy,
            run_powershell_get_sharing_policy, run_powershell_get_spoof_intelligence_insight,
            run_powershell_get_transport_rule, run_powershell_get_user_vip,
    };
    use data_ingester_splunk::splunk::{set_ssphp_run, Splunk, ToHecEvents};
    use data_ingester_supporting::keyvault::{get_keyvault_secrets, Secrets};    
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

    #[tokio::test]
    async fn test_run_powershell_get_safe_attachment_policy() -> Result<()> {
        let (splunk, secrets) = setup().await?;
        let result = run_powershell_get_safe_attachment_policy(&secrets).await?;
        splunk.send_batch((&result).to_hec_events()?).await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_run_powershell_get_atp_policy_for_o365() -> Result<()> {
        let (splunk, secrets) = setup().await?;
        let result = run_powershell_get_atp_policy_for_o365(&secrets).await?;
        splunk.send_batch((&result).to_hec_events()?).await?;
        Ok(())
    }

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
        let result = run_powershell_get_dkim_signing_config(&secrets).await?;
        splunk.send_batch((&result).to_hec_events()?).await?;
        Ok(())
    }

    #[ignore]
    #[tokio::test]
    async fn test_run_powershell_get_spoof_intelligence_insight() -> Result<()> {
        let (splunk, secrets) = setup().await?;
        let result = run_powershell_get_spoof_intelligence_insight(&secrets).await?;
        splunk.send_batch((&result).to_hec_events()?).await?;
        Ok(())
    }

    #[ignore]
    #[tokio::test]
    async fn test_run_powershell_get_blocked_sender_address() -> Result<()> {
        let (splunk, secrets) = setup().await?;
        let result = run_powershell_get_blocked_sender_address(&secrets).await?;
        splunk.send_batch((&result).to_hec_events()?).await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_run_powershell_get_email_tenant_settings() -> Result<()> {
        let (splunk, secrets) = setup().await?;
        let result = run_powershell_get_email_tenant_settings(&secrets).await?;
        splunk.send_batch((&result).to_hec_events()?).await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_run_powershell_get_user_vip() -> Result<()> {
        let (splunk, secrets) = setup().await?;
        let result = run_powershell_get_user_vip(&secrets).await?;
        splunk.send_batch((&result).to_hec_events()?).await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_run_powershell_get_protection_alert() -> Result<()> {
        let (splunk, secrets) = setup().await?;
        let result = run_powershell_get_protection_alert(&secrets).await?;
        splunk.send_batch((&result).to_hec_events()?).await?;
        Ok(())
    }

    // Breaks on JSON parsing due to nested objects
    #[ignore]
    #[tokio::test]
    async fn test_run_powershell_get_mailbox() -> Result<()> {
        let (splunk, secrets) = setup().await?;
        let result = run_powershell_get_mailbox(&secrets).await?;
        splunk.send_batch((&result).to_hec_events()?).await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_run_powershell_eop_protection_policy_rule() -> Result<()> {
        let (splunk, secrets) = setup().await?;
        let result = run_powershell_get_eop_protection_policy_rule(&secrets).await?;
        splunk.send_batch((&result).to_hec_events()?).await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_run_powershell_get_cs_teams_client_configuration() -> Result<()> {
        let (splunk, secrets) = setup().await?;
        let result = run_powershell_get_cs_teams_client_configuration(&secrets).await?;
        splunk.send_batch((&result).to_hec_events()?).await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_run_powershell_get_hosted_content_filter_policy() -> Result<()> {
        let (splunk, secrets) = setup().await?;
        let result = run_powershell_get_hosted_content_filter_policy(&secrets).await?;
        splunk.send_batch((&result).to_hec_events()?).await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_run_powershell_get_management_role_assignment() -> Result<()> {
        let (splunk, secrets) = setup().await?;
        let result = run_powershell_get_management_role_assignment(&secrets).await?;
        splunk.send_batch((&result).to_hec_events()?).await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_run_powershell_exchange_login_test() -> Result<()> {
        let (splunk, secrets) = setup().await?;
        let result = run_powershell_exchange_login_test(&secrets).await?;
        splunk.send_batch((&result).to_hec_events()?).await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_run_powershell_teams_cs_tenant_federation_configuration() -> Result<()> {
        let (splunk, secrets) = setup().await?;
        let result = run_powershell_get_cs_tenant_federation_configuration(&secrets).await?;
        splunk.send_batch((&result).to_hec_events()?).await?;
        Ok(())
    }
}
