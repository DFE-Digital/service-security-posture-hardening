use crate::ado_dev_ops_client::{AzureDevOpsClient, AzureDevOpsClientMethods};
use crate::ado_metadata::{AdoMetadata, AdoMetadataBuilder, NoRestDocs, NoType, NoUrl};
use crate::ado_response::{AddAdoResponse, AdoPaging, AdoRateLimiting, AdoResponse};
use crate::data::organization::Organizations;
use anyhow::{Context, Result};
use serde::de::DeserializeOwned;
use serde::Deserialize;
use tracing::{debug, error, trace};

pub(crate) struct AzureDevOpsClientOauth {
    pub(crate) client: reqwest::Client,
    token: Token,
    api_version: String,
    pub(crate) tenant_id: String,
}

impl AzureDevOpsClientMethods for AzureDevOpsClientOauth {}

impl AzureDevOpsClientOauth {
    pub(crate) async fn new(client_id: &str, client_secret: &str, tenant_id: &str) -> Result<Self> {
        let client = reqwest::Client::new();
        let url = format!("https://login.microsoftonline.com/{tenant_id}/oauth2/v2.0/token");
        let params = [
            ("client_id", client_id),
            ("client_secret", client_secret),
            ("grant_type", "client_credentials"),
            // Fixed ADO scope
            ("scope", "499b84ac-1321-427f-aa17-267ca6975798/.default"),
        ];
        let response = client.post(url).form(&params).send().await?;

        let token = response
            .json()
            .await
            .context("Getting JSON from Oauth request")?;

        Ok(Self {
            client,
            api_version: "7.2-preview.1".into(),
            token,
            tenant_id: tenant_id.into(),
        })
    }

    pub(crate) async fn organizations_list(&self) -> Result<Organizations> {
        let url = format!(
            "https://aexprodcus1.vsaex.visualstudio.com/_apis/EnterpriseCatalog/Organizations?tenantId={}",
            self.tenant_id
        );

        let response = self
            .client
            .get(&url)
            .bearer_auth(&self.token.access_token)
            .send()
            .await?;

        if !response.status().is_success() {
            error!(name="Azure Dev Ops", operation="organizations_list GET request", error="Non 2xx status code", status=?response.status(), headers=?response.headers());
            anyhow::bail!("failed request");
        }

        let rate_limit = AdoRateLimiting::from_headers(response.headers());
        trace!(rate_limit=?rate_limit);

        let ado_metadata = AdoMetadataBuilder::new()
            .tenant(&self.tenant_id)
            .url(&url)
            .r#type("fn organizations_list")
            .rest_docs("no REST docs")
            .build();

        let text = response.text().await?;

        let organizations = Organizations::from_csv(&text, ado_metadata);

        Ok(organizations)
    }
}

#[derive(Debug, Deserialize)]
struct Token {
    #[allow(unused)]
    token_type: TokenType,
    #[allow(unused)]
    expires_in: usize,
    #[allow(unused)]
    ext_expires_in: usize,
    access_token: String,
}

#[derive(Debug, Deserialize)]
enum TokenType {
    Bearer,
}

impl AzureDevOpsClient for AzureDevOpsClientOauth {
    fn api_version(&self) -> &str {
        self.api_version.as_str()
    }

    fn ado_metadata_builder(&self) -> AdoMetadataBuilder<NoUrl, NoType, NoRestDocs> {
        AdoMetadataBuilder::new().tenant(&self.tenant_id)
    }

    async fn get<T: DeserializeOwned + AddAdoResponse>(
        &self,
        metadata: AdoMetadata,
    ) -> Result<AdoResponse> {
        let mut continuation_token = AdoPaging::default();
        let mut collection = AdoResponse::from_metadata(metadata);

        loop {
            let next_url = if continuation_token.has_more() {
                format!(
                    "{}&continuationToken={}",
                    collection.metadata.url(),
                    continuation_token.next_token()
                )
            } else {
                collection.metadata.url().to_string()
            };

            let response = self
                .client
                .get(&next_url)
                .bearer_auth(&self.token.access_token)
                .send()
                .await?;

            let status = response.status();
            let headers = response.headers().clone();
            let text = response.text().await?;

            if !status.is_success() {
                error!(name="Azure Dev Ops", operation="GET request", error="Non 2xx status code",
                       status=?status,
                       //headers=?headers,
                       body=text,
                       url=next_url);
                anyhow::bail!("Azure Dev Org request failed with with Non 2xx status code");
            }
            collection.metadata.status.push(status.into());

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

        Ok(collection)
    }
}
