use crate::ado_dev_ops_client::{AzureDevOpsClient, AzureDevOpsClientMethods};
use crate::ado_metadata::{AdoMetadata, AdoMetadataBuilder, NoRestDocs, NoType, NoUrl};
use crate::ado_response::{AddAdoResponse, AdoPaging, AdoRateLimiting, AdoResponse};
use crate::data::organization::Organizations;
use anyhow::Result;
use serde::de::DeserializeOwned;
use tracing::{debug, error, trace};

pub(crate) struct AzureDevOpsClientPat {
    pub(crate) client: reqwest::Client,
    #[allow(unused)]
    organization: String,
    pat: String,
    api_version: String,
}

impl AzureDevOpsClientMethods for AzureDevOpsClientPat {}

impl AzureDevOpsClientPat {
    pub(crate) fn new<S1: Into<String>, S2: Into<String>>(
        organization: S1,
        pat: S2,
    ) -> Result<Self> {
        let client = reqwest::Client::new();

        Ok(Self {
            client,
            api_version: "7.2-preview.1".into(),
            pat: pat.into(),
            organization: organization.into(),
        })
    }

    #[allow(unused)]
    pub(crate) async fn organizations_list(&self) -> Result<Organizations> {
        anyhow::bail!("Not implemented for AzureDevOpsPat");
    }
}

impl AzureDevOpsClient for AzureDevOpsClientPat {
    fn api_version(&self) -> &str {
        self.api_version.as_str()
    }

    fn ado_metadata_builder(&self) -> AdoMetadataBuilder<NoUrl, NoType, NoRestDocs> {
        AdoMetadataBuilder::new()
    }

    async fn get<T: DeserializeOwned + AddAdoResponse>(
        &self,
        mut metadata: AdoMetadata,
    ) -> Result<AdoResponse> {
        let mut continuation_token = AdoPaging::default();
        let mut collection = AdoResponse::default();

        loop {
            let next_url = if continuation_token.has_more() {
                format!(
                    "{}&continuationToken={}",
                    metadata.url(),
                    continuation_token.next_token()
                )
            } else {
                metadata.url().to_string()
            };

            let response = self
                .client
                .get(&next_url)
                .basic_auth("", Some(&self.pat))
                .send()
                .await?;

            let status = response.status();
            let headers = response.headers().clone();
            let text = response.text().await?;

            if !status.is_success() {
                error!(name="Azure Dev Ops", operation="GET request", error="Non 2xx status code", status=?status, headers=?headers, body=text);
                anyhow::bail!("Azure Dev Org request failed with with Non 2xx status code");
            }
            metadata.status.push(status.into());

            let rate_limit = AdoRateLimiting::from_headers(&headers);
            debug!(rate_limit=?rate_limit);

            continuation_token = AdoPaging::from_headers(&headers);

            trace!(
                name = "Azure Dev Ops",
                operation = "get response",
                url = next_url,
                response = text
            );

            let ado_response: T = serde_json::from_str(&text)?;

            collection.count += ado_response.count();
            collection.value.extend(ado_response.values());

            if continuation_token.is_empty() {
                break;
            }
        }
        collection.metadata = Some(metadata);

        Ok(collection)
    }
}
