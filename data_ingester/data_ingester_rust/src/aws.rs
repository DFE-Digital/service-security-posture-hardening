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
use aws_sdk_iam::types::{AccessKeyMetadata, PasswordPolicy};
use serde::{Deserialize, Serialize};

use crate::aws_entities_for_policy::EntitiesForPolicyOutput;
use crate::aws_policy::Policies;
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

    try_collect_send(
        "aws_1_13_ensure_there_is_only_one_active_access_key_available_for_any_single_iam_user",
        aws_client
            .aws_1_13_ensure_there_is_only_one_active_access_key_available_for_any_single_iam_user(
            ),
        &splunk,
    )
    .await?;

    try_collect_send(
        "aws_1_15_ensure_iam_users_receive_permissions_only_through_groups",
        aws_client.aws_1_15_ensure_iam_users_receive_permissions_only_through_groups(),
        &splunk,
    )
    .await?;

    try_collect_send(
        "aws_1_16_ensure_iam_policies_that_allow_full_administrative_privileges_are_not_attached",
        aws_client.aws_1_16_ensure_iam_policies_that_allow_full_administrative_privileges_are_not_attached(),
        &splunk,
    )
        .await?;

    try_collect_send(
        "aws_1_17_ensure_a_support_role_has_been_created_to_manage_incidents_with_aws_support",
        aws_client
            .aws_1_17_ensure_a_support_role_has_been_created_to_manage_incidents_with_aws_support(),
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

    pub(crate) async fn aws_1_13_ensure_there_is_only_one_active_access_key_available_for_any_single_iam_user(
        &self,
    ) -> Result<AccessKeys> {
        let config = self.config().await?;
        let client = aws_sdk_iam::Client::new(&config);
        let users: Result<Vec<aws_sdk_iam::types::User>, _> = client
            .list_users()
            .into_paginator()
            .items()
            .send()
            .collect()
            .await;
        let mut access_keys: Vec<AccessKeyMetadataSerde> = vec![];
        for user in users? {
            let keys = client
                .list_access_keys()
                .user_name(user.user_name())
                .send()
                .await?;
            for key in keys.access_key_metadata {
                access_keys.push(key.into());
            }
        }
        Ok(AccessKeys { inner: access_keys })
    }

    pub(crate) async fn aws_1_15_ensure_iam_users_receive_permissions_only_through_groups(
        &self,
    ) -> Result<Users> {
        let config = self.config().await?;
        let client = aws_sdk_iam::Client::new(&config);
        let users: Result<Vec<aws_sdk_iam::types::User>, _> = client
            .list_users()
            .into_paginator()
            .items()
            .send()
            .collect()
            .await;

        let mut users: Users = users?.into();
        for user in &mut users.inner {
            user.attached_policies = client
                .list_attached_user_policies()
                .user_name(&user.user_name)
                .send()
                .await?
                .attached_policies
                .map(|policies| policies.into_iter().map(|policy| policy.into()).collect())
                .unwrap_or_default();

            user.policies = client
                .list_user_policies()
                .user_name(&user.user_name)
                .send()
                .await?
                .policy_names;
        }

        Ok(users)
    }

    pub(crate) async fn aws_1_16_ensure_iam_policies_that_allow_full_administrative_privileges_are_not_attached(
        &self,
    ) -> Result<Policies> {
        let config = self.config().await?;
        let client = aws_sdk_iam::Client::new(&config);
        let policies: Result<Vec<aws_sdk_iam::types::Policy>, _> = client
            .list_policies()
            .set_only_attached(Some(true))
            .into_paginator()
            .items()
            .send()
            .collect()
            .await;

        let mut policies: Policies = policies?.into();

        for policy in policies.inner.iter_mut() {
            let policy_version = client
                .get_policy_version()
                .set_policy_arn(policy.arn.clone())
                .set_version_id(policy.default_version_id.clone())
                .send()
                .await?;

            let document = policy_version
                .policy_version()
                .and_then(|policy_version| policy_version.document());

            policy.full_admin_permissions = if let Some(document) = document {
                let decoded = urlencoding::decode(document).expect("UTF-8");
                let json_document: crate::aws_policy::AwsPolicy = serde_json::from_str(&decoded)?;

                match &json_document.statement {
                    crate::aws_policy::Statement::StatementVec(vec) => Some(
                        vec.iter()
                            .any(|statement| statement.contains_full_permissions()),
                    ),
                    crate::aws_policy::Statement::StatementElement(statement) => {
                        Some(statement.contains_full_permissions())
                    }
                }
            } else {
                Some(false)
            };
        }

        Ok(policies)
    }

    pub(crate) async fn aws_1_17_ensure_a_support_role_has_been_created_to_manage_incidents_with_aws_support(
        &self,
    ) -> Result<EntitiesForPolicyOutput> {
        let config = self.config().await?;
        let client = aws_sdk_iam::Client::new(&config);
        let arn = Some("arn:aws:iam::aws:policy/AWSSupportAccess".to_string());
        let mut entities_for_policy: EntitiesForPolicyOutput = client
            .list_entities_for_policy()
            .set_policy_arn(arn.clone())
            .send()
            .await?
            .into();
        entities_for_policy.arn = arn;
        Ok(entities_for_policy)
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct Users {
    inner: Vec<User>,
}

impl From<Vec<aws_sdk_iam::types::User>> for Users {
    fn from(value: Vec<aws_sdk_iam::types::User>) -> Self {
        Self {
            inner: value.into_iter().map(|u| u.into()).collect(),
        }
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct User {
    arn: String,
    path: String,
    user_name: String,
    user_id: String,
    create_date: f64,
    attached_policies: Vec<AttachedPolicy>,
    policies: Vec<String>,
    // password_last_used: Option<f64>,
    // pub password_last_used: ::std::option::Option<::aws_smithy_types::DateTime>,
    // pub permissions_boundary: ::std::option::Option<crate::types::AttachedPermissionsBoundary>,
    // pub tags: ::std::option::Option<::std::vec::Vec<crate::types::Tag>>,
}

impl ToHecEvents for &Users {
    type Item = User;

    fn source(&self) -> &str {
        "iam_ListUsers"
    }

    fn sourcetype(&self) -> &str {
        "ssphp:aws:json"
    }

    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        Box::new(self.inner.iter())
    }
}

impl From<aws_sdk_iam::types::User> for User {
    fn from(value: aws_sdk_iam::types::User) -> Self {
        Self {
            arn: value.arn,
            path: value.path,
            user_name: value.user_name,
            user_id: value.user_id,
            create_date: value.create_date.as_secs_f64(),
            attached_policies: vec![],
            policies: vec![],
        }
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct AttachedPolicy {
    policy_name: Option<String>,
    policy_arn: Option<String>,
}

impl From<aws_sdk_iam::types::AttachedPolicy> for AttachedPolicy {
    fn from(value: aws_sdk_iam::types::AttachedPolicy) -> Self {
        Self {
            policy_arn: value.policy_arn,
            policy_name: value.policy_name,
        }
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct AccessKeys {
    inner: Vec<AccessKeyMetadataSerde>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct AccessKeyMetadataSerde {
    user_name: Option<String>,
    access_key_id: Option<String>,
    status: Option<String>,
    create_date: Option<f64>,
}

impl From<AccessKeyMetadata> for AccessKeyMetadataSerde {
    fn from(value: AccessKeyMetadata) -> Self {
        let status = match value.status {
            Some(s) => match s {
                aws_sdk_iam::types::StatusType::Active => Some("Active".to_string()),
                aws_sdk_iam::types::StatusType::Inactive => Some("InActive".to_string()),
                // aws_sdk_iam::types::StatusType::Unknown(_) => Some("Unknown".to_string()),
                _ => None,
            },
            None => None,
        };
        Self {
            user_name: value.user_name,
            access_key_id: value.access_key_id,
            status,
            create_date: value.create_date.map(|date| date.as_secs_f64()),
        }
    }
}

impl ToHecEvents for &AccessKeys {
    type Item = AccessKeyMetadataSerde;

    fn source(&self) -> &str {
        "iam_ListAccessKeys"
    }

    fn sourcetype(&self) -> &str {
        "ssphp:aws:json"
    }

    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        Box::new(self.inner.iter())
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

    #[tokio::test]
    async fn test_aws_1_13() -> Result<()> {
        let (splunk, aws) = setup().await?;
        let result = aws
            .aws_1_13_ensure_there_is_only_one_active_access_key_available_for_any_single_iam_user()
            .await?;
        splunk.send_batch((&result).to_hec_events()?).await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_aws_1_15() -> Result<()> {
        let (splunk, aws) = setup().await?;
        let result = aws
            .aws_1_15_ensure_iam_users_receive_permissions_only_through_groups()
            .await?;

        splunk.send_batch((&result).to_hec_events()?).await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_aws_1_16() -> Result<()> {
        let (splunk, aws) = setup().await?;
        let result = aws
            .aws_1_16_ensure_iam_policies_that_allow_full_administrative_privileges_are_not_attached()
            .await?;

        splunk.send_batch((&result).to_hec_events()?).await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_aws_1_17() -> Result<()> {
        let (splunk, aws) = setup().await?;
        let result = aws
            .aws_1_17_ensure_a_support_role_has_been_created_to_manage_incidents_with_aws_support()
            .await?;
        splunk.send_batch((&result).to_hec_events()?).await?;
        Ok(())
    }
}
