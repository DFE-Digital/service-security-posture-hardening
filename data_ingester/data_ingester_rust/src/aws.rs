use std::collections::HashMap;
use std::iter;
use std::sync::Arc;

use anyhow::Result;
use aws_config::meta::region::RegionProviderChain;
use aws_config::{BehaviorVersion, SdkConfig};
use aws_credential_types::provider::SharedCredentialsProvider;
use aws_sdk_account::config::{Credentials, ProvideCredentials};
use aws_sdk_account::types::ContactInformation;
use aws_sdk_iam::operation::get_account_summary::GetAccountSummaryOutput;
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
        dbg!(&response);
        let out = AccountSummary::from(response);
        Ok(out)
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
}
