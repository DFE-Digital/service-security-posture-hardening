use std::collections::HashMap;
use std::iter;
use std::sync::Arc;

use anyhow::{bail, Context, Result};
use aws_config::meta::region::RegionProviderChain;
use aws_config::{BehaviorVersion, SdkConfig};
use aws_credential_types::provider::SharedCredentialsProvider;
use aws_sdk_account::config::{Credentials, ProvideCredentials};
use aws_sdk_account::types::ContactInformation;
use aws_sdk_iam::operation::get_account_summary::GetAccountSummaryOutput;
use aws_sdk_iam::types::PasswordPolicy;
use serde::{Deserialize, Serialize};

use crate::keyvault::Secrets;
use crate::ms_graph::try_collect_send;
use crate::splunk::{set_ssphp_run, Splunk, ToHecEvents};

pub async fn aws(secrets: Arc<Secrets>, splunk: Arc<Splunk>) -> Result<()> {
    set_ssphp_run()?;

    splunk.log("Starting AWS collection").await?;
    splunk
        .log(&format!("GIT_HASH: {}", env!("GIT_HASH")))
        .await?;

    let aws_client = AwsClient { secrets };

    try_collect_send(
        "aws_1_1_maintain_current_contact_details",
        aws_client.aws_1_1_maintain_current_contact_details(),
        &splunk,
    )
    .await?;

    try_collect_send(
        "aws_1_4_ensure_no_root_user_account_access_key_exists()",
        aws_client.aws_1_4_ensure_no_root_user_account_access_key_exists(),
        &splunk,
    )
    .await?;

    try_collect_send(
        "aws_1_8_ensure_iam_password_policy_requires_minimum_length_of_14",
        aws_client.aws_1_8_ensure_iam_password_policy_requires_minimum_length_of_14(),
        &splunk,
    )
    .await?;

    try_collect_send(
        "aws_1_10_ensure_mfa_is_enabled_for_all_iam_users_that_have_a_console_password",
        aws_client.aws_1_10_ensure_mfa_is_enabled_for_all_iam_users_that_have_a_console_password(),
        &splunk,
    )
    .await?;

    splunk.log("AWS Collection Complete").await?;

    Ok(())
}

impl ProvideCredentials for AwsSecrets {
    fn provide_credentials<'a>(
        &'a self,
    ) -> aws_credential_types::provider::future::ProvideCredentials<'a>
    where
        Self: 'a,
    {
        aws_credential_types::provider::future::ProvideCredentials::new(self.load_credentials())
    }
}

#[derive(Debug)]
struct AwsSecrets {
    secrets: Arc<Secrets>,
}

impl AwsSecrets {
    async fn load_credentials(&self) -> aws_credential_types::provider::Result {
        Ok(Credentials::new(
            self.secrets.aws_access_key_id.clone(),
            self.secrets.aws_secret_access_key.clone(),
            None,
            None,
            "StaticCredentials",
        ))
    }
}

#[derive(Debug)]
struct AwsClient {
    secrets: Arc<Secrets>,
}

impl AwsClient {
    async fn config(&self) -> Result<SdkConfig> {
        let region_provider = RegionProviderChain::default_provider().or_else("us-east-1");

        let config = aws_config::defaults(BehaviorVersion::latest())
            .credentials_provider(SharedCredentialsProvider::new(AwsSecrets {
                secrets: self.secrets.clone(),
            }))
            .region(region_provider)
            .load()
            .await;
        Ok(config)
    }
    /// IAM: account:GetContactInformation
    /// https://docs.aws.amazon.com/accounts/latest/reference/API_GetContactInformation.html
    pub(crate) async fn aws_1_1_maintain_current_contact_details(
        &self,
    ) -> Result<ContactInformationSerde> {
        let config = self.config().await?;
        let client = aws_sdk_account::Client::new(&config);
        let contact_information = client.get_contact_information().send().await?;
        let out = ContactInformationSerde::from(
            contact_information.to_owned().contact_information.unwrap(),
        );
        Ok(out)
    }

    /// https://docs.aws.amazon.com/IAM/latest/APIReference/API_GetAccountSummary.html
    /// https://docs.rs/aws-sdk-iam/latest/aws_sdk_iam/client/struct.Client.html#method.get_account_summary
    pub(crate) async fn aws_1_4_ensure_no_root_user_account_access_key_exists(
        &self,
    ) -> Result<AccountSummary> {
        let config = self.config().await?;
        let client = aws_sdk_iam::Client::new(&config);
        let response = client.get_account_summary().send().await?;
        let out = AccountSummary::from(response);
        Ok(out)
    }

    /// 1.8
    /// 1.9
    pub(crate) async fn aws_1_8_ensure_iam_password_policy_requires_minimum_length_of_14(
        &self,
    ) -> Result<AccountPasswordPolicy> {
        let config = self.config().await?;
        let client = aws_sdk_iam::Client::new(&config);
        let out = match client.get_account_password_policy().send().await {
            Ok(response) => {
                AccountPasswordPolicy::from(response.password_policy.context("No password policy")?)
            }
            Err(_) => AccountPasswordPolicy::default(),
        };
        Ok(out)
    }

    pub(crate) async fn aws_1_10_ensure_mfa_is_enabled_for_all_iam_users_that_have_a_console_password(
        &self,
    ) -> Result<CredentialReport> {
        let config = self.config().await?;
        let client = aws_sdk_iam::Client::new(&config);

        loop {
            let generate_report = client.generate_credential_report().send().await?;
            match generate_report.state {
                Some(state) => match state {
                    aws_sdk_iam::types::ReportStateType::Complete => break,
                    aws_sdk_iam::types::ReportStateType::Inprogress => continue,
                    aws_sdk_iam::types::ReportStateType::Started => continue,
                    _ => bail!("Unknown Generate report state"),
                },
                None => continue,
            }
        }

        let report = client.get_credential_report().send().await?;

        let credential_report =
            CredentialReport::from(report.content.context("No credential report")?.as_ref());

        Ok(credential_report)
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct ContactInformationSerde {
    full_name: String,
    address_line1: String,
    address_line2: Option<String>,
    address_line3: Option<String>,
    city: String,
    state_or_region: Option<String>,
    district_or_county: Option<String>,
    postal_code: String,
    country_code: String,
    phone_number: String,
    company_name: Option<String>,
    website_url: Option<String>,
}

impl From<ContactInformation> for ContactInformationSerde {
    fn from(value: ContactInformation) -> Self {
        Self {
            full_name: value.full_name,
            address_line1: value.address_line1,
            address_line2: value.address_line2,
            address_line3: value.address_line3,
            city: value.city,
            state_or_region: value.state_or_region,
            district_or_county: value.district_or_county,
            postal_code: value.postal_code,
            country_code: value.country_code,
            phone_number: value.phone_number,
            company_name: value.company_name,
            website_url: value.website_url,
        }
    }
}

impl ToHecEvents for &ContactInformationSerde {
    type Item = Self;

    fn source(&self) -> &str {
        "accounts_GetContactInformation"
    }

    fn sourcetype(&self) -> &str {
        "ssphp:aws:json"
    }

    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        Box::new(iter::once(self))
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub(crate) struct AccountSummary {
    pub summary_map: ::std::collections::HashMap<String, i32>,
}

impl From<GetAccountSummaryOutput> for AccountSummary {
    fn from(value: GetAccountSummaryOutput) -> Self {
        let mut summary_map = HashMap::new();
        for (k, v) in value.summary_map().unwrap() {
            summary_map.insert(k.as_str().to_owned(), *v);
        }

        Self { summary_map }
    }
}

impl ToHecEvents for &AccountSummary {
    type Item = Self;

    fn source(&self) -> &str {
        "iam_GetAccountSummary"
    }

    fn sourcetype(&self) -> &str {
        "ssphp:aws:json"
    }

    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        Box::new(iter::once(self))
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub(crate) struct AccountPasswordPolicy {
    minimum_password_length: Option<i32>,
    require_symbols: bool,
    require_numbers: bool,
    require_uppercase_characters: bool,
    require_lowercase_characters: bool,
    allow_users_to_change_password: bool,
    expire_passwords: bool,
    max_password_age: Option<i32>,
    password_reuse_prevention: Option<i32>,
    hard_expiry: Option<bool>,
}

impl From<aws_sdk_iam::types::PasswordPolicy> for AccountPasswordPolicy {
    fn from(value: PasswordPolicy) -> Self {
        Self {
            minimum_password_length: value.minimum_password_length,
            require_symbols: value.require_symbols,
            require_numbers: value.require_numbers,
            require_uppercase_characters: value.require_uppercase_characters,
            require_lowercase_characters: value.require_lowercase_characters,
            allow_users_to_change_password: value.allow_users_to_change_password,
            expire_passwords: value.expire_passwords,
            max_password_age: value.max_password_age,
            password_reuse_prevention: value.password_reuse_prevention,
            hard_expiry: value.hard_expiry,
        }
    }
}

impl ToHecEvents for &AccountPasswordPolicy {
    type Item = Self;

    fn source(&self) -> &str {
        "iam_GetAccountPasswordPolicy"
    }

    fn sourcetype(&self) -> &str {
        "ssphp:aws:json"
    }

    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        Box::new(iter::once(self))
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub(crate) struct CredentialReport {
    inner: Vec<CredentialReportUser>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
//#[serde(rename_all = "PascalCase")]
pub(crate) struct CredentialReportUser {
    user: String,
    arn: String,
    user_creation_time: String,
    password_enabled: bool,
    password_last_used: String,
    password_last_changed: String,
    password_next_rotation: String,
    mfa_active: bool,
    access_key_1_active: bool,
    access_key_1_last_rotated: String,
    access_key_1_last_used_date: String,
    access_key_1_last_used_region: String,
    access_key_1_last_used_service: String,
    access_key_2_active: bool,
    access_key_2_last_rotated: String,
    access_key_2_last_used_date: String,
    access_key_2_last_used_region: String,
    access_key_2_last_used_service: String,
    cert_1_active: bool,
    cert_1_last_rotated: String,
    cert_2_active: bool,
    cert_2_last_rotated: String,
}

impl From<&[u8]> for CredentialReport {
    fn from(value: &[u8]) -> Self {
        let users = csv::Reader::from_reader(value)
            .deserialize::<CredentialReportUser>()
            .filter_map(|r| r.ok())
            .collect();
        Self { inner: users }
    }
}

impl ToHecEvents for &CredentialReport {
    type Item = CredentialReportUser;

    fn source(&self) -> &str {
        "iam_GetCredentialReport"
    }

    fn sourcetype(&self) -> &str {
        "ssphp:aws:json"
    }

    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        Box::new(self.inner.iter())
    }
}

#[cfg(test)]
mod test {
    use std::env;
    use std::sync::Arc;

    use super::{aws, AwsClient};
    use crate::splunk::ToHecEvents;
    use crate::{
        keyvault::get_keyvault_secrets,
        splunk::{set_ssphp_run, Splunk},
    };
    use anyhow::Result;

    pub async fn setup() -> Result<(Splunk, AwsClient)> {
        let secrets = get_keyvault_secrets(&env::var("KEY_VAULT_NAME")?).await?;
        set_ssphp_run()?;
        let splunk = Splunk::new(&secrets.splunk_host, &secrets.splunk_token)?;
        let aws = AwsClient {
            secrets: Arc::new(secrets),
        };
        Ok((splunk, aws))
    }

    #[tokio::test]
    async fn test_aws_full() -> Result<()> {
        let secrets = get_keyvault_secrets(&env::var("KEY_VAULT_NAME")?).await?;
        set_ssphp_run()?;
        let splunk = Splunk::new(&secrets.splunk_host, &secrets.splunk_token)?;
        aws(Arc::new(secrets), Arc::new(splunk)).await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_aws_config() -> Result<()> {
        let (_splunk, _aws) = setup().await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_aws_1_1() -> Result<()> {
        let (splunk, aws) = setup().await?;
        let result = aws.aws_1_1_maintain_current_contact_details().await?;
        splunk.send_batch((&result).to_hec_events()?).await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_aws_1_4() -> Result<()> {
        let (splunk, aws) = setup().await?;
        let result = aws
            .aws_1_4_ensure_no_root_user_account_access_key_exists()
            .await?;
        splunk.send_batch((&result).to_hec_events()?).await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_aws_1_8() -> Result<()> {
        let (splunk, aws) = setup().await?;
        let result = aws
            .aws_1_8_ensure_iam_password_policy_requires_minimum_length_of_14()
            .await?;
        splunk.send_batch((&result).to_hec_events()?).await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_aws_1_10() -> Result<()> {
        let (splunk, aws) = setup().await?;
        let result = aws
            .aws_1_10_ensure_mfa_is_enabled_for_all_iam_users_that_have_a_console_password()
            .await?;
        splunk.send_batch((&result).to_hec_events()?).await?;
        Ok(())
    }
}
