use std::collections::HashMap;
use std::iter;
use std::net::Ipv4Addr;
use std::str::FromStr;
use std::sync::Arc;

use aws_config::Region;
use hickory_proto::rr::record_type::RecordType;
use hickory_proto::rr::RData;
use hickory_resolver::config::*;
use hickory_resolver::Name;
use hickory_resolver::TokioAsyncResolver;

use anyhow::{bail, Context, Result};
use aws_config::meta::region::RegionProviderChain;
use aws_config::{BehaviorVersion, SdkConfig};
use aws_credential_types::provider::SharedCredentialsProvider;
use aws_sdk_account::config::{Credentials, ProvideCredentials};
use aws_sdk_account::types::ContactInformation;
use aws_sdk_iam::operation::get_account_summary::GetAccountSummaryOutput;
use aws_sdk_iam::types::{AccessKeyMetadata, PasswordPolicy};
use aws_sdk_kms::types::KeyListEntry;
use serde::{Deserialize, Serialize};
use tracing::error;
use tracing::info;

use crate::aws_alternate_contact_information::AlternateContact;
use crate::aws_config::DescribeConfigurationRecordersOutput;
use crate::aws_ec2::{DescribeFlowLogs, DescribeVpcs, FlowLog, Vpc};
use crate::aws_entities_for_policy::EntitiesForPolicyOutput;
use crate::aws_iam::Groups;
use crate::aws_iam::Users;
use crate::aws_iam::{MfaDevices, VirtualMfaDevices};
use crate::aws_kms::{KeyMetadata, KeyMetadatas};
use crate::aws_policy::Policies;
use crate::aws_route53::{HostedZone, HostedZones};
use crate::aws_s3::{
    GetBucketAclOutput, GetBucketAclOutputs, GetBucketLoggingOutput, GetBucketLoggingOutputs,
    GetBucketPolicyOutput, GetBucketPolicyOutputs, GetBucketVersioningOutput,
    GetBucketVersioningOutputs, GetPublicAccessBlockOutput, GetPublicAccessBlocks,
};
use crate::aws_securityhub::DescribeHubOutput;
use crate::aws_trail::{TrailWrapper, TrailWrappers};
use data_ingester_splunk::splunk::try_collect_send;
use data_ingester_splunk::splunk::{set_ssphp_run, Splunk, ToHecEvents};
use data_ingester_supporting::keyvault::Secrets;

pub async fn aws(secrets: Arc<Secrets>, splunk: Arc<Splunk>) -> Result<()> {
    set_ssphp_run("aws")?;

    info!("Starting AWS collection");
    info!("GIT_HASH: {}", env!("GIT_HASH"));

    let aws_client = AwsClient { secrets };

    let _ = try_collect_send(
        "aws_1_1_maintain_current_contact_details",
        aws_client.aws_1_1_maintain_current_contact_details(),
        &splunk,
    )
    .await;

    let _ = try_collect_send(
        "aws_1_2_ensure_security_contact_information_is_registered",
        aws_client.aws_1_2_ensure_security_contact_information_is_registered(),
        &splunk,
    )
    .await;

    let _ = try_collect_send(
        "aws_1_4_ensure_no_root_user_account_access_key_exists()",
        aws_client.aws_1_4_ensure_no_root_user_account_access_key_exists(),
        &splunk,
    )
    .await;

    let _ = try_collect_send(
        "aws_1_8_ensure_iam_password_policy_requires_minimum_length_of_14",
        aws_client.aws_1_8_ensure_iam_password_policy_requires_minimum_length_of_14(),
        &splunk,
    )
    .await;

    let _ = try_collect_send(
        "aws_1_6_ensure_hardware_mfa_is_enabled_for_the_root_user_account",
        aws_client.aws_1_6_ensure_hardware_mfa_is_enabled_for_the_root_user_account(),
        &splunk,
    )
    .await;

    let _ = try_collect_send(
        "aws_1_10_ensure_mfa_is_enabled_for_all_iam_users_that_have_a_console_password",
        aws_client.aws_1_10_ensure_mfa_is_enabled_for_all_iam_users_that_have_a_console_password(),
        &splunk,
    )
    .await;

    let _ = try_collect_send(
        "aws_1_13_ensure_there_is_only_one_active_access_key_available_for_any_single_iam_user",
        aws_client
            .aws_1_13_ensure_there_is_only_one_active_access_key_available_for_any_single_iam_user(
            ),
        &splunk,
    )
    .await;

    let _ = try_collect_send(
        "aws_1_15_ensure_iam_users_receive_permissions_only_through_groups",
        aws_client.aws_1_15_ensure_iam_users_receive_permissions_only_through_groups(),
        &splunk,
    )
    .await;

    let _ = try_collect_send(
        "aws_1_16_ensure_iam_policies_that_allow_full_administrative_privileges_are_not_attached",
        aws_client.aws_1_16_ensure_iam_policies_that_allow_full_administrative_privileges_are_not_attached(),
        &splunk,
    )
        .await;

    let _ = try_collect_send(
        "aws_1_17_ensure_a_support_role_has_been_created_to_manage_incidents_with_aws_support",
        aws_client
            .aws_1_17_ensure_a_support_role_has_been_created_to_manage_incidents_with_aws_support(),
        &splunk,
    )
    .await;

    let _ = try_collect_send(
        "aws_1_19_ensure_that_all_the_expired_tls_certificates_stored_in_aws_iam_are_removed",
        aws_client
            .aws_1_19_ensure_that_all_the_expired_tls_certificates_stored_in_aws_iam_are_removed(),
        &splunk,
    )
    .await;

    let _ = try_collect_send(
        "aws_1_20_ensure_that_iam_access_analyzer_is_enabled_for_all_regions",
        aws_client.aws_1_20_ensure_that_iam_access_analyzer_is_enabled_for_all_regions(),
        &splunk,
    )
    .await;

    let _ = try_collect_send(
        "aws_1_22_ensure_access_to_awscloudshellfullaccess_is_restricted",
        aws_client.aws_1_22_ensure_access_to_awscloudshellfullaccess_is_restricted(),
        &splunk,
    )
    .await;

    let _ = try_collect_send(
        "aws_2_1_1_ensure_s3_bucket_policy_is_set_to_deny_http_requests",
        aws_client.aws_2_1_1_ensure_s3_bucket_policy_is_set_to_deny_http_requests(),
        &splunk,
    )
    .await;

    let _ = try_collect_send(
        "aws_2_1_2_ensure_mfa_delete_is_enabled_on_s3_buckets",
        aws_client.aws_2_1_2_ensure_mfa_delete_is_enabled_on_s3_buckets(),
        &splunk,
    )
    .await;

    let _ = try_collect_send(
        "aws_2_1_4_ensure_that_s3_buckets_are_configured_with_block_public_access",
        aws_client.aws_2_1_4_ensure_that_s3_buckets_are_configured_with_block_public_access(),
        &splunk,
    )
    .await;

    let _ = try_collect_send(
        "aws_2_1_4_ensure_that_s3_buckets_are_configured_with_block_public_access_accounts",
        aws_client
            .aws_2_1_4_ensure_that_s3_buckets_are_configured_with_block_public_access_accounts(),
        &splunk,
    )
    .await;

    let _ = try_collect_send(
        "aws_3_1_ensure_cloudtrail_is_enabled_in_all_regions",
        aws_client.aws_3_1_ensure_cloudtrail_is_enabled_in_all_regions(),
        &splunk,
    )
    .await;

    let _ = try_collect_send(
        "aws_3_3_ensure_the_s3_bucket_used_to_store_cloudtrail_logs_is_not_publicly_accessible_acl",
        aws_client
            .aws_3_3_ensure_the_s3_bucket_used_to_store_cloudtrail_logs_is_not_publicly_accessible_acl(
            ),
        &splunk,
    )
    .await;

    let _ = try_collect_send(
        "aws_3_3_ensure_the_s3_bucket_used_to_store_cloudtrail_logs_is_not_publicly_accessible_bucket_policy",
        aws_client
            .aws_3_3_ensure_the_s3_bucket_used_to_store_cloudtrail_logs_is_not_publicly_accessible_bucket_policy(
            ),
        &splunk,
    )
        .await;

    let _ = try_collect_send(
        "aws_3_5_ensure_aws_config_is_enabled_in_all_regions",
        aws_client.aws_3_5_ensure_aws_config_is_enabled_in_all_regions(),
        &splunk,
    )
    .await;

    let _ = try_collect_send(
        "aws_3_6_ensure_s3_bucket_access_logging_is_enabled_on_the_cloudtrail_s3_bucket",
        aws_client.aws_3_6_ensure_s3_bucket_access_logging_is_enabled_on_the_cloudtrail_s3_bucket(),
        &splunk,
    )
    .await;

    let _ = try_collect_send(
        "aws_3_8_ensure_rotation_for_customer_created_symmetric_cmks_is_enabled",
        aws_client.aws_3_8_ensure_rotation_for_customer_created_symmetric_cmks_is_enabled(),
        &splunk,
    )
    .await;

    let _ = try_collect_send(
        "aws_3_9_ensure_vpc_flow_logging_is_enabled_in_all_vpcs_vpc",
        aws_client.aws_3_9_ensure_vpc_flow_logging_is_enabled_in_all_vpcs_vpc(),
        &splunk,
    )
    .await;

    let _ = try_collect_send(
        "aws_3_9_ensure_vpc_flow_logging_is_enabled_in_all_vpcs_flow_logs",
        aws_client.aws_3_9_ensure_vpc_flow_logging_is_enabled_in_all_vpcs_flow_logs(),
        &splunk,
    )
    .await;

    let _ = try_collect_send(
        "aws_4_16_ensure_aws_security_hub_is_enabled",
        aws_client.aws_4_16_ensure_aws_security_hub_is_enabled(),
        &splunk,
    )
    .await;

    let _ = try_collect_send("aws_dfe_1x", aws_client.aws_dfe_1x(), &splunk).await;

    let _ = try_collect_send("aws_dfe_2x", aws_client.aws_dfe_2x(), &splunk).await;

    let _ = try_collect_send("aws_dfe_3x", aws_client.aws_dfe_3x(), &splunk).await;

    let zones = try_collect_send("aws_dfe_4x", aws_client.aws_dfe_4x(), &splunk)
        .await
        .context("AWS 4x: Getting zones")?;

    let _ = try_collect_send("aws_dfe_5x", aws_client.aws_dfe_5x(zones), &splunk).await;

    info!("AWS Collection Complete");

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
    aws_access_key_id: String,
    aws_secret_access_key: String,
}

impl AwsSecrets {
    async fn load_credentials(&self) -> aws_credential_types::provider::Result {
        Ok(Credentials::new(
            self.aws_access_key_id.clone(),
            self.aws_secret_access_key.clone(),
            None,
            None,
            "StaticCredentials",
        ))
    }
}

struct AwsClient {
    secrets: Arc<Secrets>,
}

impl AwsClient {
    async fn config(&self) -> Result<SdkConfig> {
        let region_provider = RegionProviderChain::default_provider().or_else("us-east-1");

        let config = aws_config::defaults(BehaviorVersion::latest())
            .credentials_provider(SharedCredentialsProvider::new(AwsSecrets {
                aws_access_key_id: self
                    .secrets
                    .aws_access_key_id
                    .as_ref()
                    .context("Expect aws_access_key_id secret")?
                    .clone(),
                aws_secret_access_key: self
                    .secrets
                    .aws_secret_access_key
                    .as_ref()
                    .context("Expect aws_secret_access_key secret")?
                    .clone(),
            }))
            .region(region_provider)
            .load()
            .await;
        Ok(config)
    }

    /// Generate an AWS config for a specific region
    async fn config_for_region(&self, region: &str) -> Result<SdkConfig> {
        let region_provider = RegionProviderChain::first_try(Region::new(region.to_string()));

        let config = aws_config::defaults(BehaviorVersion::latest())
            .credentials_provider(SharedCredentialsProvider::new(AwsSecrets {
                aws_access_key_id: self
                    .secrets
                    .aws_access_key_id
                    .as_ref()
                    .context("Expect aws_access_key_id secret")?
                    .clone(),
                aws_secret_access_key: self
                    .secrets
                    .aws_secret_access_key
                    .as_ref()
                    .context("Expect aws_secret_access_key secret")?
                    .clone(),
            }))
            .region(region_provider)
            .load()
            .await;
        Ok(config)
    }

    /// Get an up to date list of regions from EC2, or use a static list of regions.
    #[allow(dead_code)]
    async fn list_of_regions(&self) -> Result<Vec<String>> {
        let config = self
            .config()
            .await
            .context("Can't build default AWS config")?;
        let ec2_client = aws_sdk_ec2::Client::new(&config);
        let regions = match ec2_client.describe_regions().all_regions(true).send().await {
            Ok(regions) => regions
                .regions
                .unwrap_or_default()
                .into_iter()
                .flat_map(|region| region.region_name)
                .collect(),
            Err(err) => {
                error!("Unable to get list of regions from EC2 endpoint: {:?}", err);
                error!("Using static list of regions");
                vec![
                    "af-south-1".to_string(),
                    "ap-east-1".to_string(),
                    "ap-northeast-1".to_string(),
                    "ap-northeast-2".to_string(),
                    "ap-northeast-3".to_string(),
                    "ap-south-1".to_string(),
                    "ap-southeast-1".to_string(),
                    "ap-southeast-2".to_string(),
                    "ap-southeast-3".to_string(),
                    "ca-central-1".to_string(),
                    "cn-north-1".to_string(),
                    "cn-northwest-1".to_string(),
                    "eu-central-1".to_string(),
                    "eu-north-1".to_string(),
                    "eu-south-1".to_string(),
                    "eu-west-1".to_string(),
                    "eu-west-2".to_string(),
                    "eu-west-3".to_string(),
                    "me-south-1".to_string(),
                    "sa-east-1".to_string(),
                    "us-east-1".to_string(),
                    "us-east-2".to_string(),
                    "us-gov-east-1".to_string(),
                    "us-gov-west-1".to_string(),
                    "us-west-1".to_string(),
                    "us-west-2".to_string(),
                ]
            }
        };

        Ok(regions)
    }

    /// configs for all regions
    #[allow(dead_code)]
    async fn config_all_regions(&self) -> Result<Vec<SdkConfig>> {
        let regions = self
            .list_of_regions()
            .await
            .context("Unable to get regions")?;
        let mut configs = Vec::with_capacity(regions.len());
        for region in regions {
            let config = self
                .config_for_region(&region)
                .await
                .context(format!("Failed to get config for: {}", &region))?;
            configs.push(config);
        }
        Ok(configs)
    }

    async fn client_for_bucket(
        &self,
        s3_client: &aws_sdk_s3::Client,
        bucket_name: &str,
    ) -> Result<aws_sdk_s3::Client> {
        let bucket_location = s3_client
            .get_bucket_location()
            .set_bucket(Some(bucket_name.to_string()))
            .send()
            .await
            .context(format!(
                "Getting s3_client.get_bucket_location for bucket: {}",
                bucket_name
            ))?
            .location_constraint()
            .map(|lc| lc.as_str().to_owned())
            .unwrap_or_else(|| "us-east-1".to_owned());

        let bucket_region = aws_config::Region::new(bucket_location);

        let region_provider = RegionProviderChain::first_try(bucket_region);

        let bucket_client_config = aws_config::defaults(BehaviorVersion::latest())
            .credentials_provider(SharedCredentialsProvider::new(AwsSecrets {
                aws_access_key_id: self
                    .secrets
                    .aws_access_key_id
                    .as_ref()
                    .context("Expect aws_access_key_id secret")?
                    .clone(),
                aws_secret_access_key: self
                    .secrets
                    .aws_secret_access_key
                    .as_ref()
                    .context("Expect aws_secret_access_key secret")?
                    .clone(),
            }))
            .region(region_provider)
            .load()
            .await;

        Ok(aws_sdk_s3::Client::new(&bucket_client_config))
    }

    /// IAM: account:GetContactInformation
    /// https://docs.aws.amazon.com/accounts/latest/reference/API_GetContactInformation.html
    pub(crate) async fn aws_1_1_maintain_current_contact_details(
        &self,
    ) -> Result<ContactInformationSerde> {
        let config = self.config().await?;
        let client = aws_sdk_account::Client::new(&config);
        let contact_information = client.get_contact_information().send().await?;
        let out = contact_information
            .contact_information
            .map(|ci| ci.into())
            .unwrap_or_default();
        Ok(out)
    }

    pub(crate) async fn aws_1_2_ensure_security_contact_information_is_registered(
        &self,
    ) -> Result<AlternateContact> {
        let config = self.config().await?;
        let client = aws_sdk_account::Client::new(&config);
        let alternate_contact_information = match client
            .get_alternate_contact()
            .set_alternate_contact_type(Some(
                aws_sdk_account::types::AlternateContactType::Security,
            ))
            .send()
            .await
        {
            Ok(ac) => ac.alternate_contact.map(|ac| ac.into()).unwrap_or_default(),
            Err(_) => AlternateContact::default(),
        };
        Ok(alternate_contact_information)
    }

    pub(crate) async fn aws_1_6_ensure_hardware_mfa_is_enabled_for_the_root_user_account(
        &self,
    ) -> Result<VirtualMfaDevices> {
        let config = self.config().await?;
        let client = aws_sdk_iam::Client::new(&config);
        let virtual_mfa = client
            .list_virtual_mfa_devices()
            .into_paginator()
            .items()
            .send()
            .collect::<Result<Vec<aws_sdk_iam::types::VirtualMfaDevice>, _>>()
            .await?
            .into_iter()
            .map(|vmd| vmd.into())
            .collect();
        Ok(VirtualMfaDevices { inner: virtual_mfa })
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
        let config = self.config().await.context("Building config")?;
        let client = aws_sdk_iam::Client::new(&config);
        let users: Result<Vec<aws_sdk_iam::types::User>, _> = client
            .list_users()
            .into_paginator()
            .items()
            .send()
            .collect()
            .await;
        let users = users.context("Getting Users")?;
        let mut access_keys: Vec<AccessKeyMetadataSerde> = vec![];
        for user in users {
            let keys = client
                .list_access_keys()
                .user_name(user.user_name())
                .send()
                .await
                .context("Getting access keys")?;
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

    pub(crate) async fn aws_1_19_ensure_that_all_the_expired_tls_certificates_stored_in_aws_iam_are_removed(
        &self,
    ) -> Result<ServerCertificatesMetadata> {
        let config = self.config().await?;
        let client = aws_sdk_iam::Client::new(&config);
        let server_certificates: ServerCertificatesMetadata = client
            .list_server_certificates()
            .send()
            .await?
            .server_certificate_metadata_list
            .into();

        Ok(server_certificates)
    }

    /// TODO Might need to check other regions
    pub(crate) async fn aws_1_20_ensure_that_iam_access_analyzer_is_enabled_for_all_regions(
        &self,
    ) -> Result<AnalyzerSummaries> {
        let config = self.config().await?;
        let client = aws_sdk_accessanalyzer::Client::new(&config);
        let access_analyzers: AnalyzerSummaries =
            client.list_analyzers().send().await?.analyzers.into();

        Ok(access_analyzers)
    }

    pub(crate) async fn aws_1_22_ensure_access_to_awscloudshellfullaccess_is_restricted(
        &self,
    ) -> Result<EntitiesForPolicyOutput> {
        let config = self.config().await?;
        let client = aws_sdk_iam::Client::new(&config);
        let arn = Some("arn:aws:iam::aws:policy/AWSCloudShellFullAccess".to_string());
        let mut entities_for_policy: EntitiesForPolicyOutput = client
            .list_entities_for_policy()
            .set_policy_arn(arn.clone())
            .send()
            .await?
            .into();
        entities_for_policy.arn = arn;
        Ok(entities_for_policy)
    }

    pub(crate) async fn aws_2_1_1_ensure_s3_bucket_policy_is_set_to_deny_http_requests(
        &self,
    ) -> Result<GetBucketPolicyOutputs> {
        let config = self.config().await?;

        let s3_client = aws_sdk_s3::Client::new(&config);
        let buckets = s3_client.list_buckets().send().await?;

        let mut policies = vec![];

        for bucket in buckets.buckets.unwrap_or_default().into_iter() {
            let bucket_name = bucket.name;

            let bucket_client = self
                .client_for_bucket(
                    &s3_client,
                    bucket_name.as_ref().context("Bucket should have name")?,
                )
                .await
                .context("Building client for bucket")?;

            match bucket_client
                .get_bucket_policy()
                .set_bucket(bucket_name.clone())
                .send()
                .await
            {
                Ok(policy) => {
                    let mut policy: GetBucketPolicyOutput = policy.into();
                    policy.bucket_name = bucket_name;
                    policies.push(policy);
                }
                Err(e) => {
                    error!("Error getting bucket policy: {:?} {:?}", bucket_name, e);
                    policies.push(GetBucketPolicyOutput {
                        policy: None,
                        bucket_name,
                        trail_arn: None,
                    })
                }
            }
        }
        Ok(GetBucketPolicyOutputs { inner: policies })
    }

    pub(crate) async fn aws_2_1_2_ensure_mfa_delete_is_enabled_on_s3_buckets(
        &self,
    ) -> Result<GetBucketVersioningOutputs> {
        let config = self.config().await?;

        let s3_client = aws_sdk_s3::Client::new(&config);
        let buckets = s3_client.list_buckets().send().await?;

        let mut versionings = vec![];

        for bucket in buckets.buckets.unwrap_or_default().into_iter() {
            let bucket_name = bucket.name;

            let bucket_client = self
                .client_for_bucket(
                    &s3_client,
                    bucket_name.as_ref().context("Bucket should have name")?,
                )
                .await
                .context("Building client for bucket")?;

            match bucket_client
                .get_bucket_versioning()
                .set_bucket(bucket_name.clone())
                .send()
                .await
            {
                Ok(versioning) => {
                    let mut versioning: GetBucketVersioningOutput = versioning.into();
                    versioning.bucket_name.clone_from(&bucket_name);
                    versionings.push(versioning);
                }
                Err(e) => {
                    error!("Error getting bucket policy: {:?} {:?}", bucket_name, e);
                    versionings.push(GetBucketVersioningOutput {
                        status: None,
                        mfa_delete: None,
                        bucket_name,
                    })
                }
            }
        }
        Ok(GetBucketVersioningOutputs { inner: versionings })
    }

    pub(crate) async fn aws_2_1_4_ensure_that_s3_buckets_are_configured_with_block_public_access(
        &self,
    ) -> Result<GetPublicAccessBlocks> {
        let config = self.config().await?;

        let s3_client = aws_sdk_s3::Client::new(&config);
        let buckets = s3_client.list_buckets().send().await?;

        let mut blocks = vec![];

        for bucket in buckets.buckets.unwrap_or_default().into_iter() {
            let bucket_name = bucket.name;

            let bucket_client = self
                .client_for_bucket(
                    &s3_client,
                    bucket_name.as_ref().context("Bucket should have name")?,
                )
                .await
                .context("Building client for bucket")?;

            match bucket_client
                .get_public_access_block()
                .set_bucket(bucket_name.clone())
                .send()
                .await
            {
                Ok(block) => {
                    let mut block: GetPublicAccessBlockOutput = block.into();
                    block.bucket_name = bucket_name;
                    blocks.push(block);
                }
                Err(e) => {
                    error!("Error getting bucket policy: {:?} {:?}", bucket_name, &e);
                    blocks.push(GetPublicAccessBlockOutput {
                        public_access_block_configuration: None,
                        bucket_name,
                    });
                }
            }
        }
        Ok(GetPublicAccessBlocks { inner: blocks })
    }

    pub(crate) async fn aws_2_1_4_ensure_that_s3_buckets_are_configured_with_block_public_access_accounts(
        &self,
    ) -> Result<crate::aws_s3control::GetPublicAccessBlockOutput> {
        let config = self.config().await?;

        let s3control_client = aws_sdk_s3control::Client::new(&config);
        let sts_client = aws_sdk_sts::Client::new(&config);

        let account_id = sts_client.get_caller_identity().send().await?.account;
        let public_access_block = match s3control_client
            .get_public_access_block()
            .set_account_id(account_id.clone())
            .send()
            .await
        {
            Ok(pab) => {
                let mut pab: crate::aws_s3control::GetPublicAccessBlockOutput = pab.into();
                pab.account_id = account_id;
                pab
            }
            Err(e) => {
                error!(
                    "Error getting public access block for: {:?} {:?}",
                    account_id, e
                );
                crate::aws_s3control::GetPublicAccessBlockOutput {
                    public_access_block_configuration: None,
                    account_id,
                }
            }
        };
        Ok(public_access_block)
    }

    pub(crate) async fn aws_3_1_ensure_cloudtrail_is_enabled_in_all_regions(
        &self,
    ) -> Result<TrailWrappers> {
        let config = self.config().await?;
        let client = aws_sdk_cloudtrail::Client::new(&config);
        let trails = client.describe_trails().send().await?;

        let mut trail_wrappers = vec![];

        for trail in trails.trail_list.unwrap_or_default().into_iter() {
            trail_wrappers.push(TrailWrapper {
                trail,
                trail_status: None,
                event_selectors: None,
            });

            let trail_wrapper = trail_wrappers.last_mut().expect("Just pushed onto vec");

            let region = match &trail_wrapper.trail.home_region() {
                Some(region) => region.to_string(),
                None => continue,
            };
            let config = self.config_for_region(&region).await?;
            let client = aws_sdk_cloudtrail::Client::new(&config);

            let name = match &trail_wrapper.trail.name() {
                Some(name) => Some(name.to_string()),
                None => continue,
            };

            let trail_status = client
                .get_trail_status()
                .set_name(name.clone())
                .send()
                .await
                .ok();

            trail_wrapper.trail_status = trail_status.map(|ts| ts.into());

            let event_selectors = client
                .get_event_selectors()
                .set_trail_name(name)
                .send()
                .await
                .ok();

            trail_wrapper.event_selectors = event_selectors.map(|es| es.into());
        }

        Ok(TrailWrappers {
            inner: trail_wrappers,
        })
    }

    pub(crate) async fn aws_3_3_ensure_the_s3_bucket_used_to_store_cloudtrail_logs_is_not_publicly_accessible_acl(
        &self,
    ) -> Result<GetBucketAclOutputs> {
        let config = self.config().await?;
        let trail_client = aws_sdk_cloudtrail::Client::new(&config);
        let trails = trail_client.describe_trails().send().await?;

        let s3_client = aws_sdk_s3::Client::new(&config);

        let mut acls = vec![];
        for trail in trails.trail_list.unwrap_or_default().into_iter() {
            let bucket_name = trail.s3_bucket_name;

            let bucket_client = match self
                .client_for_bucket(
                    &s3_client,
                    bucket_name.as_ref().context("Bucket should have name")?,
                )
                .await
                .context("Building client for bucket")
            {
                Ok(client) => client,
                Err(err) => {
                    error!(
                        "Error building client for bucket: {:?}, {:?}",
                        &bucket_name, err
                    );
                    continue;
                }
            };

            match bucket_client
                .get_bucket_acl()
                .set_bucket(bucket_name.clone())
                .send()
                .await
            {
                Ok(acl) => {
                    let mut bucket_acl: GetBucketAclOutput = acl.into();
                    bucket_acl.trail_arn = trail.trail_arn;
                    bucket_acl.bucket_name.clone_from(&bucket_name);
                    acls.push(bucket_acl);
                }

                Err(e) => {
                    error!("Error getting bucket acl: {:?} {:?}", bucket_name, e);
                }
            }
        }
        Ok(GetBucketAclOutputs { inner: acls })
    }

    pub(crate) async fn aws_3_3_ensure_the_s3_bucket_used_to_store_cloudtrail_logs_is_not_publicly_accessible_bucket_policy(
        &self,
    ) -> Result<GetBucketPolicyOutputs> {
        let config = self.config().await?;
        let trail_client = aws_sdk_cloudtrail::Client::new(&config);
        let trails = trail_client.describe_trails().send().await?;

        let s3_client = aws_sdk_s3::Client::new(&config);

        let mut policies = vec![];
        for trail in trails.trail_list.unwrap_or_default().into_iter() {
            let bucket_name = trail.s3_bucket_name;

            let bucket_client = match self
                .client_for_bucket(
                    &s3_client,
                    bucket_name.as_ref().context("Bucket should have name")?,
                )
                .await
                .context("Building client for bucket")
            {
                Ok(client) => client,
                Err(err) => {
                    error!(
                        "Error building client for bucket: {:?}, {:?}",
                        &bucket_name, err
                    );
                    continue;
                }
            };

            match bucket_client
                .get_bucket_policy()
                .set_bucket(bucket_name.clone())
                .send()
                .await
            {
                Ok(policy) => {
                    let mut policy: GetBucketPolicyOutput = policy.into();
                    policy.bucket_name.clone_from(&bucket_name);
                    policy.trail_arn = trail.trail_arn;
                    policies.push(policy);
                }
                Err(e) => {
                    error!("Error getting bucket policy: {:?} {:?}", bucket_name, e);
                }
            }
        }
        Ok(GetBucketPolicyOutputs { inner: policies })
    }

    pub(crate) async fn aws_3_6_ensure_s3_bucket_access_logging_is_enabled_on_the_cloudtrail_s3_bucket(
        &self,
    ) -> Result<GetBucketLoggingOutputs> {
        let config = self.config().await.context("building confing")?;
        let trail_client = aws_sdk_cloudtrail::Client::new(&config);
        let trails = trail_client
            .describe_trails()
            .send()
            .await
            .context("AWS describe_trails")?;

        let s3_client = aws_sdk_s3::Client::new(&config);

        let mut logging_policies = vec![];
        for trail in trails.trail_list.unwrap_or_default().into_iter() {
            let bucket_name = trail.s3_bucket_name;

            let bucket_client = match self
                .client_for_bucket(
                    &s3_client,
                    bucket_name.as_ref().context("Bucket should have name")?,
                )
                .await
                .context("Building client for bucket")
            {
                Ok(client) => client,
                Err(err) => {
                    error!(
                        "Error building client for bucket: {:?}, {:?}",
                        &bucket_name, err
                    );
                    continue;
                }
            };

            match bucket_client
                .get_bucket_logging()
                .set_bucket(bucket_name.clone())
                .send()
                .await
            {
                Ok(logging) => {
                    let mut logging: GetBucketLoggingOutput = logging.into();
                    logging.bucket_name.clone_from(&bucket_name);
                    logging.trail_arn = trail.trail_arn;
                    logging_policies.push(logging);
                }
                Err(e) => {
                    error!("Error getting bucket policy: {:?} {:?}", bucket_name, e);
                }
            }
        }
        Ok(GetBucketLoggingOutputs {
            inner: logging_policies,
        })
    }

    pub(crate) async fn aws_3_5_ensure_aws_config_is_enabled_in_all_regions(
        &self,
    ) -> Result<DescribeConfigurationRecordersOutput> {
        let config = self.config().await?;
        let config_client = aws_sdk_config::Client::new(&config);
        let mut configs: DescribeConfigurationRecordersOutput = config_client
            .describe_configuration_recorders()
            .send()
            .await?
            .into();

        if configs.configuration_recorders.is_none() {
            return Ok(DescribeConfigurationRecordersOutput {
                configuration_recorders: None,
            });
        }

        for config in configs
            .configuration_recorders
            .as_mut()
            .expect("Should have configuration recorders")
            .iter_mut()
        {
            match config_client
                .describe_configuration_recorder_status()
                .set_configuration_recorder_names(
                    config.name.as_ref().map(|name| vec![name.clone()]),
                )
                .send()
                .await
            {
                Ok(status) => {
                    config.status = status
                        .configuration_recorders_status
                        .map(|vec| vec.into_iter().map(|crs| crs.into()).collect());
                }
                Err(e) => {
                    error!("Error getting bucket policy: {:?} {:?}", config.name, e);
                }
            }
        }
        Ok(configs)
    }

    pub(crate) async fn aws_3_9_ensure_vpc_flow_logging_is_enabled_in_all_vpcs_vpc(
        &self,
    ) -> Result<DescribeVpcs> {
        let config = self.config().await?;
        let ec2_client = aws_sdk_ec2::Client::new(&config);
        let vpcs: Vec<Vpc> = ec2_client
            .describe_vpcs()
            .into_paginator()
            .items()
            .send()
            .collect::<Result<Vec<aws_sdk_ec2::types::Vpc>, _>>()
            .await?
            .into_iter()
            .map(|vpc| vpc.into())
            .collect();
        Ok(DescribeVpcs { inner: vpcs })
    }

    pub(crate) async fn aws_3_9_ensure_vpc_flow_logging_is_enabled_in_all_vpcs_flow_logs(
        &self,
    ) -> Result<DescribeFlowLogs> {
        let config = self.config().await?;
        let ec2_client = aws_sdk_ec2::Client::new(&config);
        let flow_logs: Vec<FlowLog> = ec2_client
            .describe_flow_logs()
            .into_paginator()
            .items()
            .send()
            .collect::<Result<Vec<aws_sdk_ec2::types::FlowLog>, _>>()
            .await?
            .into_iter()
            .map(|vpc| vpc.into())
            .collect();
        Ok(DescribeFlowLogs { inner: flow_logs })
    }

    pub(crate) async fn aws_3_8_ensure_rotation_for_customer_created_symmetric_cmks_is_enabled(
        &self,
    ) -> Result<KeyMetadatas> {
        let config = self.config().await?;
        let client = aws_sdk_kms::Client::new(&config);
        let mut key_list_entries: Vec<KeyListEntry> = client
            .list_keys()
            .into_paginator()
            .items()
            .send()
            .collect::<Result<Vec<KeyListEntry>, _>>()
            .await?;

        let mut customer_key_metadatas: Vec<KeyMetadata> = vec![];
        for key in key_list_entries.iter_mut() {
            let describe_key = client
                .describe_key()
                .set_key_id(key.key_id.clone())
                .send()
                .await?;

            let key_manager_type_is_customer = describe_key
                .key_metadata()
                .and_then(|km| km.key_manager.as_ref())
                .is_some_and(|km| matches!(km, aws_sdk_kms::types::KeyManagerType::Customer));

            let key_spec_is_symmetric_default = describe_key
                .key_metadata()
                .and_then(|km| km.key_spec.as_ref())
                .is_some_and(|ks| matches!(ks, aws_sdk_kms::types::KeySpec::SymmetricDefault));

            match (key_manager_type_is_customer, key_spec_is_symmetric_default) {
                (true, true) => {
                    let mut describe_key_metadata: KeyMetadata = match describe_key.key_metadata {
                        Some(km) => km.into(),
                        None => continue,
                    };
                    match client
                        .get_key_rotation_status()
                        .set_key_id(key.key_id.clone())
                        .send()
                        .await
                    {
                        Ok(krs) => {
                            describe_key_metadata.key_rotation_status = Some(krs.into());
                        }
                        Err(_) => continue,
                    }
                    customer_key_metadatas.push(describe_key_metadata);
                }
                (_, _) => continue,
            }
        }

        Ok(KeyMetadatas {
            inner: customer_key_metadatas,
        })
    }

    pub(crate) async fn aws_4_16_ensure_aws_security_hub_is_enabled(
        &self,
    ) -> Result<DescribeHubOutput> {
        let config = self.config().await?;
        let client = aws_sdk_securityhub::Client::new(&config);
        let hubs: DescribeHubOutput = client.describe_hub().send().await?.into();

        Ok(hubs)
    }

    // TODO: correct usecase ID
    pub(crate) async fn aws_dfe_1x(&self) -> Result<MfaDevices> {
        let config = self.config().await?;
        let client = aws_sdk_iam::Client::new(&config);

        let user_names: Vec<String> = client
            .list_users()
            .into_paginator()
            .items()
            .send()
            .collect::<Result<Vec<aws_sdk_iam::types::User>, _>>()
            .await
            .context("Getting Users")?
            .into_iter()
            .map(|user| user.user_name)
            .collect();

        let mut all_mfa_devices = vec![];
        for user_name in user_names {
            let mfa_devices = client
                .list_mfa_devices()
                .set_user_name(Some(user_name))
                .into_paginator()
                .items()
                .send()
                .collect::<Result<Vec<aws_sdk_iam::types::MfaDevice>, _>>()
                .await?
                .into_iter()
                .map(|md| md.into());
            //                .collect::<Vec<crate::aws_iam::MfaDevice>>();

            all_mfa_devices.extend(mfa_devices);
        }

        Ok(MfaDevices {
            inner: all_mfa_devices,
        })
    }

    pub(crate) async fn aws_dfe_2x(&self) -> Result<Groups> {
        let config = self.config().await?;
        let client = aws_sdk_iam::Client::new(&config);
        let mut groups: Vec<crate::aws_iam::Group> = client
            .list_groups()
            .into_paginator()
            .items()
            .send()
            .collect::<Result<Vec<aws_sdk_iam::types::Group>, _>>()
            .await?
            .into_iter()
            .map(|group| group.into())
            .collect();

        for group in &mut groups {
            group.users = client
                .get_group()
                .group_name(&group.group_name)
                .send()
                .await?
                .users()
                .iter()
                .map(|user| user.arn().to_string())
                .collect();

            group.attached_policies = client
                .list_attached_group_policies()
                .group_name(&group.group_name)
                .send()
                .await?
                .attached_policies
                .map(|policies| policies.into_iter().map(|policy| policy.into()).collect())
                .unwrap_or_default();

            group.policies = client
                .list_group_policies()
                .group_name(&group.group_name)
                .send()
                .await?
                .policy_names;
        }
        Ok(Groups { inner: groups })
    }

    // TODO: correct usecase ID
    pub(crate) async fn aws_dfe_3x(&self) -> Result<crate::aws_organizations::Organization> {
        let config = self.config().await?;
        let client = aws_sdk_organizations::Client::new(&config);

        let mut organization: crate::aws_organizations::Organization = client
            .describe_organization()
            .send()
            .await?
            .organization
            .map(|o| o.into())
            .unwrap_or_default();

        let sts_client = aws_sdk_sts::Client::new(&config);
        organization.account_id = sts_client.get_caller_identity().send().await?.account;
        Ok(organization)
    }

    // TODO: correct usecase ID
    pub(crate) async fn aws_dfe_4x(&self) -> Result<crate::aws_route53::HostedZones> {
        let config = self.config().await?;
        let client = aws_sdk_route53::Client::new(&config);
        let mut hosted_zones = client
            .list_hosted_zones()
            .into_paginator()
            .items()
            .send()
            .collect::<Result<Vec<aws_sdk_route53::types::HostedZone>, _>>()
            .await?
            .into_iter()
            .map(|hz| hz.into())
            .collect::<Vec<HostedZone>>();

        for zone in hosted_zones.iter_mut() {
            // TODO: Might want to break zone.resource_record_sets into individual Splunk events
            zone.resource_record_sets = client
                .list_resource_record_sets()
                .set_hosted_zone_id(Some(zone.id.clone()))
                .send()
                .await?
                .resource_record_sets
                .into_iter()
                .map(|rrs| Some(rrs.into()))
                .collect();
        }

        Ok(HostedZones {
            inner: hosted_zones,
        })
    }

    pub(crate) async fn aws_dfe_5x(
        &self,
        zones: crate::aws_route53::HostedZones,
    ) -> Result<RecordSets> {
        // Build resolver
        let resolver =
            TokioAsyncResolver::tokio(ResolverConfig::default(), ResolverOpts::default());

        let mut sets = vec![];

        for zone in zones.inner {
            sets.push(Route53RecordSet::new(&zone.name));

            let Some(resource_record_set) = zone.resource_record_sets else {
                sets.last_mut()
                    .expect("just pushed to vec")
                    .errors
                    .push(Route53LookupErrors {
                        context: "zone.resource_record_sets in None".to_string(),
                        error: "".to_string(),
                    });
                continue;
            };

            for resource_record in resource_record_set {
                let mut this_set = sets.last_mut().expect("Just pushed onto vec");

                // If name is Some then we've already looped once
                if this_set.name.is_some() {
                    sets.push(Route53RecordSet::new(&zone.name));
                    this_set = sets.last_mut().expect("Just pushed onto vec");
                }

                this_set.name = Some(resource_record.name.clone());

                // Convert the record type into `hickory-proto::RecordType`
                let record_type = match RecordType::from_str(resource_record.r#type.as_str()) {
                    Ok(record_type) => record_type,
                    Err(err) => {
                        this_set.errors.push(Route53LookupErrors {
                            context: format!(
                                "failed to get RecordType for {}",
                                resource_record.r#type.as_str()
                            ),
                            error: format!("{err:?}"),
                        });
                        error!(
                            "failed to get RecordType for {}: {}",
                            resource_record.r#type.as_str(),
                            err
                        );
                        continue;
                    }
                };

                this_set.record_type = Some(record_type);

                let Some(resource_records) = resource_record.resource_records else {
                    this_set.errors.push(Route53LookupErrors {
                        context: "resource_records.resource_records in None".to_string(),
                        error: "".to_string(),
                    });
                    // No records to match against so we raise an error and continue
                    continue;
                };
                this_set.route53_data.clone_from(&resource_records);

                // Perform lookup
                let lookup_name = resource_record.name.as_str();
                let results = match resolver.lookup(lookup_name, record_type).await {
                    Ok(results) => results,
                    Err(err) => {
                        this_set.errors.push(Route53LookupErrors {
                            context: format!(
                                "failed to lookup for: {} {}",
                                lookup_name, record_type
                            ),
                            error: err.to_string(),
                        });
                        error!(
                            "failed to lookup for: {} {}: {}",
                            lookup_name, record_type, err
                        );
                        continue;
                    }
                };

                for record in results.records() {
                    if record.record_type() != record_type {
                        continue;
                    }

                    this_set.dns_answers.push(Rdata {
                        value: record.clone(),
                        in_route53: false,
                    });

                    let data = match record.data() {
                        Some(data) => data,
                        None => {
                            this_set.errors.push(Route53LookupErrors {
                                context: "record.data() in None".to_string(),
                                error: "".to_string(),
                            });
                            continue;
                        }
                    };

                    let in_route53 = match data {
                        RData::A(a) => resource_records
                            .iter()
                            .filter_map(|record| Ipv4Addr::from_str(record.value.as_str()).ok())
                            .any(|ipv4| ipv4 == a.0),
                        RData::AAAA(_aaaa) => false,
                        // RData::ANAME(_) => false,
                        RData::CAA(_caa) => false,
                        // TODO
                        RData::CNAME(_cname) => false,
                        RData::CSYNC(_r) => false,
                        // RData::HINFO(_) => todo!(),
                        // RData::HTTPS(_) => todo!(),
                        RData::MX(_mx) => false,
                        // RData::NAPTR(_) => todo!(),
                        // RData::NULL(_) => todo!(),
                        RData::NS(ns) => resource_records
                            .iter()
                            .map(|record| Name::from_str(&record.value).unwrap_or_default())
                            .any(|name| name == ns.0),
                        // RData::OPENPGPKEY(_) => todo!(),
                        // RData::OPT(_) => todo!(),
                        // RData::PTR(_) => todo!(),
                        RData::SOA(soa) => {
                            let soa = format!(
                                "{} {} {} {} {} {} {}",
                                soa.mname(),
                                soa.rname(),
                                soa.serial(),
                                soa.refresh(),
                                soa.retry(),
                                soa.expire(),
                                soa.minimum(),
                            );

                            resource_records.iter().any(|value| value.value == soa)
                        }
                        RData::SRV(_srv) => false,
                        // RData::SSHFP(_) => todo!(),
                        // RData::SVCB(_) => todo!(),
                        // RData::TLSA(_) => todo!(),
                        RData::TXT(_txt) => false,
                        // RData::Unknown { code, rdata } => todo!(),
                        // RData::ZERO => todo!(),
                        &_ => false,
                    };

                    this_set
                        .dns_answers
                        .last_mut()
                        .expect("Just pushed this element element")
                        .in_route53 = in_route53;
                }
            }
        }

        sets.iter_mut().for_each(|set| {
            set.in_dns =
                set.dns_answers.iter().all(|answer| answer.in_route53) && set.errors.is_empty();
        });

        Ok(RecordSets { inner: sets })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RecordSets {
    inner: Vec<Route53RecordSet>,
}

impl ToHecEvents for &RecordSets {
    type Item = Route53RecordSet;

    fn source(&self) -> &str {
        "route53_zone_checker"
    }

    fn sourcetype(&self) -> &str {
        "ssphp:aws:json"
    }

    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        Box::new(self.inner.iter())
    }
    fn ssphp_run_key(&self) -> &str {
        "aws"
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Route53Answers {}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Route53RecordSet {
    name: Option<String>,
    record_type: Option<RecordType>,
    route53_data: Vec<crate::aws_route53::ResourceRecord>,
    dns_answers: Vec<Rdata>,
    hosted_zone_name: String,
    in_dns: bool,
    errors: Vec<Route53LookupErrors>,
}

impl Route53RecordSet {
    pub fn new(hosted_zone_name: &str) -> Self {
        Self {
            name: None,
            record_type: None,
            dns_answers: vec![],
            hosted_zone_name: hosted_zone_name.to_string(),
            in_dns: false,
            errors: vec![],
            route53_data: vec![],
        }
    }
}

// Maybe rename this??
#[derive(Debug, Clone, PartialEq, Serialize)]
pub(crate) struct Route53LookupErrors {
    context: String,
    error: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Rdata {
    value: hickory_proto::rr::resource::Record,
    in_route53: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub(crate) struct AnalyzerSummaries {
    inner: Vec<AnalyzerSummary>,
}

impl From<Vec<aws_sdk_accessanalyzer::types::AnalyzerSummary>> for AnalyzerSummaries {
    fn from(value: Vec<aws_sdk_accessanalyzer::types::AnalyzerSummary>) -> Self {
        Self {
            inner: value.into_iter().map(|u| u.into()).collect(),
        }
    }
}

impl ToHecEvents for &AnalyzerSummaries {
    type Item = AnalyzerSummary;

    fn source(&self) -> &str {
        "accessanalyzers_ListAnalyzers"
    }

    fn sourcetype(&self) -> &str {
        "ssphp:aws:json"
    }

    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        Box::new(self.inner.iter())
    }
    fn ssphp_run_key(&self) -> &str {
        "aws"
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct AnalyzerSummary {
    /// <p>The ARN of the analyzer.</p>
    pub arn: ::std::string::String,
    /// <p>The name of the analyzer.</p>
    pub name: ::std::string::String,
    /// <p>The type of analyzer, which corresponds to the zone of trust chosen for the analyzer.</p>
    pub r#type: String,
    /// <p>A timestamp for the time at which the analyzer was created.</p>
    pub created_at: f64,
    /// <p>The resource that was most recently analyzed by the analyzer.</p>
    pub last_resource_analyzed: ::std::option::Option<::std::string::String>,
    /// <p>The time at which the most recently analyzed resource was analyzed.</p>
    pub last_resource_analyzed_at: ::std::option::Option<f64>,
}

impl From<aws_sdk_accessanalyzer::types::AnalyzerSummary> for AnalyzerSummary {
    fn from(value: aws_sdk_accessanalyzer::types::AnalyzerSummary) -> Self {
        Self {
            arn: value.arn,
            name: value.name,
            r#type: value.r#type.as_str().to_string(),
            created_at: value.created_at.as_secs_f64(),
            last_resource_analyzed: value.last_resource_analyzed,
            last_resource_analyzed_at: value
                .last_resource_analyzed_at
                .map(|lraa| lraa.as_secs_f64()),
        }
    }
}

//////////////////
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct ServerCertificatesMetadata {
    inner: Vec<ServerCertificateMetadata>,
}

impl From<Vec<aws_sdk_iam::types::ServerCertificateMetadata>> for ServerCertificatesMetadata {
    fn from(value: Vec<aws_sdk_iam::types::ServerCertificateMetadata>) -> Self {
        Self {
            inner: value.into_iter().map(|u| u.into()).collect(),
        }
    }
}

impl ToHecEvents for &ServerCertificatesMetadata {
    type Item = ServerCertificateMetadata;

    fn source(&self) -> &str {
        "iam_ListServerCertificates"
    }

    fn sourcetype(&self) -> &str {
        "ssphp:aws:json"
    }

    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        Box::new(self.inner.iter())
    }
    fn ssphp_run_key(&self) -> &str {
        "aws"
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct ServerCertificateMetadata {
    pub path: ::std::string::String,
    /// <p>The name that identifies the server certificate.</p>
    pub server_certificate_name: ::std::string::String,
    /// <p>The stable and unique string identifying the server certificate. For more information about IDs, see <a href="https://docs.aws.amazon.com/IAM/latest/UserGuide/Using_Identifiers.html">IAM identifiers</a> in the <i>IAM User Guide</i>.</p>
    pub server_certificate_id: ::std::string::String,
    /// <p>The Amazon Resource Name (ARN) specifying the server certificate. For more information about ARNs and how to use them in policies, see <a href="https://docs.aws.amazon.com/IAM/latest/UserGuide/Using_Identifiers.html">IAM identifiers</a> in the <i>IAM User Guide</i>.</p>
    pub arn: ::std::string::String,
    /// <p>The date when the server certificate was uploaded.</p>
    pub upload_date: ::std::option::Option<f64>,
    /// <p>The date on which the certificate is set to expire.</p>
    pub expiration: ::std::option::Option<f64>,
}

impl From<aws_sdk_iam::types::ServerCertificateMetadata> for ServerCertificateMetadata {
    fn from(value: aws_sdk_iam::types::ServerCertificateMetadata) -> Self {
        Self {
            arn: value.arn,
            path: value.path,
            server_certificate_name: value.server_certificate_name,
            server_certificate_id: value.server_certificate_id,
            upload_date: value.upload_date.map(|ud| ud.as_secs_f64()),
            expiration: value.expiration.map(|e| e.as_secs_f64()),
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
    fn ssphp_run_key(&self) -> &str {
        "aws"
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
    fn ssphp_run_key(&self) -> &str {
        "aws"
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub(crate) struct AccountSummary {
    pub summary_map: Option<::std::collections::HashMap<String, i32>>,
}

impl From<GetAccountSummaryOutput> for AccountSummary {
    fn from(value: GetAccountSummaryOutput) -> Self {
        let summary_map = value.summary_map().map(|sm| {
            sm.iter()
                .map(|(k, v)| (k.as_str().to_owned(), *v))
                .collect::<HashMap<String, i32>>()
        });

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
    fn ssphp_run_key(&self) -> &str {
        "aws"
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
    fn ssphp_run_key(&self) -> &str {
        "aws"
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
    fn ssphp_run_key(&self) -> &str {
        "aws"
    }
}

#[cfg(feature = "live_tests")]
#[cfg(test)]
mod live_tests {
    use std::env;
    use std::sync::Arc;

    use super::AwsClient;

    #[cfg(feature = "live_tests")]
    use super::aws;

    #[cfg(feature = "live_tests")]
    use data_ingester_splunk::splunk::ToHecEvents;

    use data_ingester_splunk::splunk::{set_ssphp_run, Splunk};
    use data_ingester_supporting::keyvault::get_keyvault_secrets;

    use anyhow::{Context, Result};

    #[cfg(feature = "live_tests")]
    pub async fn setup() -> Result<(Splunk, AwsClient)> {
        let secrets = get_keyvault_secrets(&env::var("KEY_VAULT_NAME")?).await?;
        set_ssphp_run("default")?;
        let splunk = Splunk::new(
            secrets.splunk_host.as_ref().context("No value")?,
            secrets.splunk_token.as_ref().context("No value")?,
        )?;
        let aws = AwsClient {
            secrets: Arc::new(secrets),
        };
        Ok((splunk, aws))
    }

    #[cfg(feature = "live_tests")]
    #[tokio::test]
    async fn test_aws_full() -> Result<()> {
        let secrets = get_keyvault_secrets(&env::var("KEY_VAULT_NAME")?).await?;
        set_ssphp_run("default")?;
        let splunk = Splunk::new(
            secrets.splunk_host.as_ref().context("No value")?,
            secrets.splunk_token.as_ref().context("No value")?,
        )?;
        aws(Arc::new(secrets), Arc::new(splunk)).await?;
        Ok(())
    }

    #[cfg(feature = "live_tests")]
    #[tokio::test]
    async fn test_aws_1_1() -> Result<()> {
        let (splunk, aws) = setup().await?;
        let result = aws.aws_1_1_maintain_current_contact_details().await?;
        assert!(!result.full_name.is_empty());
        splunk.send_batch((&result).to_hec_events()?).await?;
        Ok(())
    }

    #[cfg(feature = "live_tests")]
    #[tokio::test]
    async fn test_aws_1_2() -> Result<()> {
        let (splunk, aws) = setup().await?;
        let result = aws
            .aws_1_2_ensure_security_contact_information_is_registered()
            .await?;
        splunk.send_batch((&result).to_hec_events()?).await?;
        Ok(())
    }

    #[cfg(feature = "live_tests")]
    #[tokio::test]
    async fn test_aws_1_4() -> Result<()> {
        let (splunk, aws) = setup().await?;
        let result = aws
            .aws_1_4_ensure_no_root_user_account_access_key_exists()
            .await?;
        assert!(!result
            .summary_map
            .as_ref()
            .map(|sm| sm.is_empty())
            .unwrap_or_else(|| true));
        splunk.send_batch((&result).to_hec_events()?).await?;
        Ok(())
    }

    #[cfg(feature = "live_tests")]
    #[tokio::test]
    async fn test_aws_1_6() -> Result<()> {
        let (splunk, aws) = setup().await?;
        let result = aws
            .aws_1_6_ensure_hardware_mfa_is_enabled_for_the_root_user_account()
            .await?;
        assert!(!result.inner.is_empty());
        splunk.send_batch((&result).to_hec_events()?).await?;
        Ok(())
    }

    #[cfg(feature = "live_tests")]
    #[tokio::test]
    async fn test_aws_1_8() -> Result<()> {
        let (splunk, aws) = setup().await?;
        let result = aws
            .aws_1_8_ensure_iam_password_policy_requires_minimum_length_of_14()
            .await?;
        splunk.send_batch((&result).to_hec_events()?).await?;
        Ok(())
    }

    #[cfg(feature = "live_tests")]
    #[tokio::test]
    async fn test_aws_1_10() -> Result<()> {
        let (splunk, aws) = setup().await?;
        let result = aws
            .aws_1_10_ensure_mfa_is_enabled_for_all_iam_users_that_have_a_console_password()
            .await?;
        assert!(!result.inner.is_empty());
        splunk.send_batch((&result).to_hec_events()?).await?;
        Ok(())
    }

    #[cfg(feature = "live_tests")]
    #[tokio::test]
    async fn test_aws_1_13() -> Result<()> {
        let (splunk, aws) = setup().await?;
        let result = aws
            .aws_1_13_ensure_there_is_only_one_active_access_key_available_for_any_single_iam_user()
            .await?;
        assert!(!result.inner.is_empty());
        splunk.send_batch((&result).to_hec_events()?).await?;
        Ok(())
    }

    #[cfg(feature = "live_tests")]
    #[tokio::test]
    async fn test_aws_1_15() -> Result<()> {
        let (splunk, aws) = setup().await?;
        let result = aws
            .aws_1_15_ensure_iam_users_receive_permissions_only_through_groups()
            .await?;
        assert!(result.inner.iter().any(|u| !u.attached_policies.is_empty()));
        assert!(result.inner.iter().any(|u| !u.policies.is_empty()));
        splunk.send_batch((&result).to_hec_events()?).await?;
        Ok(())
    }

    #[cfg(feature = "live_tests")]
    #[tokio::test]
    async fn test_aws_1_16() -> Result<()> {
        let (splunk, aws) = setup().await?;
        let result = aws
            .aws_1_16_ensure_iam_policies_that_allow_full_administrative_privileges_are_not_attached()
            .await?;
        assert!(result.inner.iter().all(|e| e.arn.is_some()));
        assert!(result
            .inner
            .iter()
            .any(|e| e.full_admin_permissions.is_some()));
        splunk.send_batch((&result).to_hec_events()?).await?;
        Ok(())
    }

    #[cfg(feature = "live_tests")]
    #[tokio::test]
    async fn test_aws_1_17() -> Result<()> {
        let (splunk, aws) = setup().await?;
        let result = aws
            .aws_1_17_ensure_a_support_role_has_been_created_to_manage_incidents_with_aws_support()
            .await?;

        assert!(result.arn.is_some());
        splunk.send_batch((&result).to_hec_events()?).await?;
        Ok(())
    }

    #[cfg(feature = "live_tests")]
    #[tokio::test]
    async fn test_aws_1_19() -> Result<()> {
        let (splunk, aws) = setup().await?;
        let result = aws
            .aws_1_19_ensure_that_all_the_expired_tls_certificates_stored_in_aws_iam_are_removed()
            .await?;
        splunk.send_batch((&result).to_hec_events()?).await?;
        Ok(())
    }

    #[cfg(feature = "live_tests")]
    #[tokio::test]
    async fn test_aws_1_20() -> Result<()> {
        let (splunk, aws) = setup().await?;
        let result = aws
            .aws_1_20_ensure_that_iam_access_analyzer_is_enabled_for_all_regions()
            .await?;
        splunk.send_batch((&result).to_hec_events()?).await?;
        Ok(())
    }

    #[cfg(feature = "live_tests")]
    #[tokio::test]
    async fn test_aws_1_22() -> Result<()> {
        let (splunk, aws) = setup().await?;
        let result = aws
            .aws_1_22_ensure_access_to_awscloudshellfullaccess_is_restricted()
            .await?;
        assert!(result.arn.is_some());
        splunk.send_batch((&result).to_hec_events()?).await?;
        Ok(())
    }

    #[cfg(feature = "live_tests")]
    #[tokio::test]
    async fn test_aws_2_1_1() -> Result<()> {
        let (splunk, aws) = setup().await?;
        let result = aws
            .aws_2_1_1_ensure_s3_bucket_policy_is_set_to_deny_http_requests()
            .await?;
        assert!(result
            .inner
            .iter()
            .all(|bucket| bucket.bucket_name.is_some()));
        assert!(result.inner.iter().any(|bucket| bucket.policy.is_some()));
        splunk.send_batch((&result).to_hec_events()?).await?;
        Ok(())
    }

    #[cfg(feature = "live_tests")]
    #[tokio::test]
    async fn test_aws_2_1_2() -> Result<()> {
        let (splunk, aws) = setup().await?;
        let result = aws
            .aws_2_1_2_ensure_mfa_delete_is_enabled_on_s3_buckets()
            .await?;
        assert!(result
            .inner
            .iter()
            .all(|bucket| bucket.bucket_name.is_some()));
        assert!(result
            .inner
            .iter()
            .any(|bucket| bucket.mfa_delete.is_some() || bucket.status.is_some()));
        splunk.send_batch((&result).to_hec_events()?).await?;
        Ok(())
    }

    #[cfg(feature = "live_tests")]
    #[tokio::test]
    async fn test_aws_2_1_4() -> Result<()> {
        let (splunk, aws) = setup().await?;
        let result = aws
            .aws_2_1_4_ensure_that_s3_buckets_are_configured_with_block_public_access()
            .await?;
        assert!(result
            .inner
            .iter()
            .all(|bucket| bucket.bucket_name.is_some()));
        assert!(result
            .inner
            .iter()
            .any(|bucket| bucket.public_access_block_configuration.is_some()));
        splunk.send_batch((&result).to_hec_events()?).await?;
        Ok(())
    }

    #[cfg(feature = "live_tests")]
    #[tokio::test]
    async fn test_aws_2_1_4_accounts() -> Result<()> {
        let (splunk, aws) = setup().await?;
        let result = aws
            .aws_2_1_4_ensure_that_s3_buckets_are_configured_with_block_public_access_accounts()
            .await?;
        assert!(result.account_id.is_some());
        assert!(result.public_access_block_configuration.is_some());
        splunk.send_batch((&result).to_hec_events()?).await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_aws_3_1() -> Result<()> {
        let (splunk, aws) = setup().await?;
        let result = aws
            .aws_3_1_ensure_cloudtrail_is_enabled_in_all_regions()
            .await?;
        splunk.send_batch((&result).to_hec_events()?).await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_aws_3_3_acl() -> Result<()> {
        let (splunk, aws) = setup().await?;
        let result = aws
            .aws_3_3_ensure_the_s3_bucket_used_to_store_cloudtrail_logs_is_not_publicly_accessible_acl()
            .await?;
        splunk.send_batch((&result).to_hec_events()?).await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_aws_3_3_bucket_policy() -> Result<()> {
        let (splunk, aws) = setup().await?;
        let result = aws
            .aws_3_3_ensure_the_s3_bucket_used_to_store_cloudtrail_logs_is_not_publicly_accessible_bucket_policy()
            .await?;
        assert!(result.inner.iter().all(|l| l.bucket_name.is_some()));
        assert!(result.inner.iter().all(|l| l.trail_arn.is_some()));
        assert!(result.inner.iter().any(|l| l.policy.is_some()));
        splunk.send_batch((&result).to_hec_events()?).await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_aws_3_5() -> Result<()> {
        let (splunk, aws) = setup().await?;
        let result = aws
            .aws_3_5_ensure_aws_config_is_enabled_in_all_regions()
            .await?;
        assert!(&result.configuration_recorders.is_some());
        assert!(&result
            .configuration_recorders
            .as_ref()
            .unwrap()
            .iter()
            .any(|r| r.status.is_some()));
        splunk.send_batch((&result).to_hec_events()?).await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_aws_3_6() -> Result<()> {
        let (splunk, aws) = setup().await?;
        let result = aws
            .aws_3_6_ensure_s3_bucket_access_logging_is_enabled_on_the_cloudtrail_s3_bucket()
            .await?;
        assert!(result.inner.iter().all(|l| l.bucket_name.is_some()));
        assert!(result.inner.iter().all(|l| l.trail_arn.is_some()));
        assert!(result.inner.iter().any(|l| l.logging_enabled.is_some()));
        splunk.send_batch((&result).to_hec_events()?).await?;
        Ok(())
    }

    #[cfg(feature = "live_tests")]
    #[tokio::test]
    async fn test_aws_3_8() -> Result<()> {
        let (splunk, aws) = setup().await?;
        let result = aws
            .aws_3_8_ensure_rotation_for_customer_created_symmetric_cmks_is_enabled()
            .await?;
        splunk.send_batch((&result).to_hec_events()?).await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_aws_3_9_vpcs() -> Result<()> {
        let (splunk, aws) = setup().await?;
        let result = aws
            .aws_3_9_ensure_vpc_flow_logging_is_enabled_in_all_vpcs_vpc()
            .await?;
        splunk.send_batch((&result).to_hec_events()?).await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_aws_3_9_flow_logs() -> Result<()> {
        let (splunk, aws) = setup().await?;
        let result = aws
            .aws_3_9_ensure_vpc_flow_logging_is_enabled_in_all_vpcs_flow_logs()
            .await?;
        splunk.send_batch((&result).to_hec_events()?).await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_aws_4_16() -> Result<()> {
        let (splunk, aws) = setup().await?;
        let result = aws.aws_4_16_ensure_aws_security_hub_is_enabled().await?;
        splunk.send_batch((&result).to_hec_events()?).await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_aws_dfe_1x() -> Result<()> {
        let (splunk, aws) = setup().await?;
        let result = aws.aws_dfe_1x().await?;
        assert!(!result.inner.is_empty());
        splunk.send_batch((&result).to_hec_events()?).await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_aws_dfe_2x() -> Result<()> {
        let (splunk, aws) = setup().await?;
        let result = aws.aws_dfe_2x().await?;
        assert!(!result.inner.is_empty());
        splunk.send_batch((&result).to_hec_events()?).await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_aws_dfe_3x() -> Result<()> {
        let (splunk, aws) = setup().await?;
        let result = aws.aws_dfe_3x().await?;
        assert!(result.account_id.is_some());
        splunk.send_batch((&result).to_hec_events()?).await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_aws_dfe_4x() -> Result<()> {
        let (splunk, aws) = setup().await?;
        let result = aws.aws_dfe_4x().await?;
        assert!(!result.inner.is_empty());
        splunk.send_batch((&result).to_hec_events()?).await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_aws_dfe_5x() -> Result<()> {
        let (splunk, aws) = setup().await?;
        let result = aws.aws_dfe_5x().await?;
        assert!(!result.inner.is_empty());
        splunk.send_batch((&result).to_hec_events()?).await?;
        Ok(())
    }
}
