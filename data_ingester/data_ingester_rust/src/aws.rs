use std::iter;

use anyhow::Result;
use aws_config::meta::region::RegionProviderChain;
use aws_config::{BehaviorVersion, SdkConfig};
use aws_sdk_account::types::ContactInformation;
use serde::{Deserialize, Serialize};

use crate::splunk::ToHecEvents;

async fn aws_config() -> Result<SdkConfig> {
    let region_provider = RegionProviderChain::default_provider().or_else("us-east-1");
    let config = aws_config::defaults(BehaviorVersion::latest())
        .region(region_provider)
        .load()
        .await;
    Ok(config)
}
/// IAM: account:GetContactInformation
/// https://docs.aws.amazon.com/accounts/latest/reference/API_GetContactInformation.html
pub(crate) async fn aws_1_1_maintain_current_contact_details() -> Result<ContactInformationSerde> {
    let config = aws_config().await?;
    let client = aws_sdk_account::Client::new(&config);
    let contact_information = client.get_contact_information().send().await?;
    let out =
        ContactInformationSerde::from(contact_information.to_owned().contact_information.unwrap());
    Ok(out)
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
        "accounts_GetContactInformation:check_with_ian"
    }

    fn sourcetype(&self) -> &str {
        "json:aws:check_with_ian"
    }

    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        Box::new(iter::once(self))
    }
}

#[cfg(test)]
mod test {
    use std::env;

    use super::aws_config;
    use crate::splunk::ToHecEvents;
    use crate::{
        aws::aws_1_1_maintain_current_contact_details,
        keyvault::get_keyvault_secrets,
        splunk::{set_ssphp_run, Splunk},
    };
    use anyhow::Result;

    pub async fn setup() -> Result<Splunk> {
        let secrets = get_keyvault_secrets(&env::var("KEY_VAULT_NAME")?).await?;
        set_ssphp_run()?;
        let splunk = Splunk::new(&secrets.splunk_host, &secrets.splunk_token)?;
        Ok(splunk)
    }

    #[tokio::test]
    async fn test_aws_config() -> Result<()> {
        aws_config().await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_aws_1_1() -> Result<()> {
        let splunk = setup().await?;
        let result = aws_1_1_maintain_current_contact_details().await?;
        splunk.send_batch((&result).to_hec_events()?).await?;
        Ok(())
    }
}
