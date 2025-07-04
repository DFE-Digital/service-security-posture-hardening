use anyhow::{Context, Result};
use async_recursion::async_recursion;
use azure_core::auth::TokenCredential;
use data_ingester_splunk::splunk::SplunkTrait;
use data_ingester_splunk::splunk::{set_ssphp_run, Splunk, ToHecEvents};
use data_ingester_supporting::keyvault::Secrets;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::{HashMap, VecDeque},
    sync::Arc,
};
use tokio::time::{Duration, Instant};
use tracing::{error, info};
pub async fn azure_resource_graph(secrets: Arc<Secrets>, splunk: Arc<Splunk>) -> Result<()> {
    set_ssphp_run("azure_resource_graph")?;

    info!("Starting Azure Resource Graph collection");
    info!("GIT_HASH: {}", env!("GIT_HASH"));

    let azure_rest = AzureRest::new(
        secrets
            .azure_client_id
            .as_ref()
            .context("Expect azure_client_id secret")?,
        secrets
            .azure_client_secret
            .as_ref()
            .context("Expect azure_client_secret secret")?,
        secrets
            .azure_tenant_id
            .as_ref()
            .context("Expect client_tenant_id secret")?,
    )
    .await
    .context("Can't build rest client")?;
    resource_graph_all(azure_rest, &splunk)
        .await
        .context("Running azure_resource_graph")?;

    Ok(())
}

async fn resource_graph_all(az_client: AzureRest, splunk: &Splunk) -> Result<()> {
    let endpoint = "https://management.azure.com/providers/Microsoft.ResourceGraph/resources?api-version=2021-03-01";
    let mut rate_limit = RateLimit::default();
    for sub in az_client.subscriptions().inner.iter() {
        let sub_id = sub.subscription_id.as_ref().context("no subscription_id")?;

        for table in &crate::resource_graph::RESOURCE_GRAPH_TABLES {
            info!("{}: {}", sub_id, table);
            let mut request_body =
                ResourceGraphRequest::new(sub_id, &format!("{} | order by name asc", &table));

            if *table == "guestconfigurationresources" {
                request_body.options.top = Some(10);
            }

            let mut response = match make_request(
                &az_client,
                endpoint,
                &request_body,
                &mut rate_limit,
            )
            .await
            {
                Ok(response) => response,
                Err(err) => {
                    error!(err=?err, table=table, "Failed making request for Azure resource graph table: {}", table);
                    continue;
                }
            };

            let events = (&response.data)
                .to_hec_events()
                .context("Serialize ResourceGraphResponse.data events")?;
            splunk
                .send_batch(events)
                .await
                .context("Sending events to Splunk")?;
            let mut batch = 0;
            while let Some(ref skip_token) = response.skip_token {
                batch += 1;
                info!("{}: {}: batch {}", sub_id, table, batch);
                request_body.add_skip_token(skip_token);

                response = make_request(&az_client, endpoint, &request_body, &mut rate_limit)
                    .await
                    .context("Failed making Resource Graph API request")?;

                let events = (&response.data)
                    .to_hec_events()
                    .context("Serialize ResourceGraphResponse.data events")?;
                splunk
                    .send_batch(events)
                    .await
                    .context("Sending events to Splunk")?;
            }
        }
        az_client
            .credential
            .clear_cache()
            .await
            .context("Clear AZ credential cache")?;
    }
    Ok(())
}

#[async_recursion]
async fn make_request(
    az_client: &AzureRest,
    endpoint: &str,
    request_body: &ResourceGraphRequest,
    rate_limit: &mut RateLimit,
) -> Result<QueryResponse> {
    let response = loop {
        rate_limit.wait().await?;

        let result = az_client
            .post_rest_request(endpoint, &request_body)
            .await
            .context("Sending Resource Graph Post Request")?;

        match result {
            // Happy path
            ResourceGraphResponse::Query(response) => break response,

            // Known errors
            ResourceGraphResponse::Error(error) => match error.error.code {
                QueryErrorErrorCode::RateLimiting => {
                    error!("Rate limited!:\n {:?}", error);
                    tokio::time::sleep(rate_limit.interval).await;
                    continue;
                }
                QueryErrorErrorCode::BadRequest => {
                    match &error
                        .error
                        .details
                        .first()
                        .context("No error details")?
                        .code
                    {
                        QueryErrorErrorDetailsCode::ResponsePayloadTooLarge => {
                            error!("ResponsePayloadTooLarge error!");
                            let mut new_request_body = request_body.clone();
                            new_request_body.options.top = Some(1);
                            break make_request(az_client, endpoint, &new_request_body, rate_limit)
                                .await
                                .context("ResonsePayloadTooLarge recovery")?;
                        }

                        QueryErrorErrorDetailsCode::RateLimiting => {
                            error!("Rate limited!:\n {:?}", error);
                            tokio::time::sleep(rate_limit.interval).await;
                            continue;
                        }

                        QueryErrorErrorDetailsCode::DisallowedLogicalTableName => {
                            error!("Disallowed Logical Table: {:?}", &request_body);
                            anyhow::bail!("Disallowed Logical Table: {:?}", &request_body);
                        }

                        // Unknown Errors and responses
                        QueryErrorErrorDetailsCode::Other(other) => {
                            error!("{:?}", &other);
                            anyhow::bail!("Unknown Error Type : {:?}", other);
                        }
                    }
                }
                // Unknown Errors and responses
                QueryErrorErrorCode::Other(other) => {
                    error!("{:?}", &other);
                    anyhow::bail!("Unknown Error Type : {:?}", other);
                }
            },
            // Unknown Errors and responses
            ResourceGraphResponse::Other(other) => {
                error!("{:?}", &other);
                anyhow::bail!("Unknown response Error: {:?}", other);
            }
        };
    };
    Ok(response)
}

use crate::azure_rest::AzureRest;
pub(crate) static RESOURCE_GRAPH_TABLES: [&str; 27] = [
    "advisorresources",
    "alertsmanagementresources",
    "appserviceresources",
    "authorizationresources",
    "chaosresources",
    "communitygalleryresources",
    "desktopvirtualizationresources",
    "edgeorderresources",
    "extendedlocationresources",
    "guestconfigurationresources",
    "healthresources",
    "iotsecurityresources",
    "kubernetesconfigurationresources",
    "maintenanceresources",
    "managedservicesresources",
    "networkresources",
    // Orbital services have been retired https://azure.microsoft.com/en-gb/updates?id=azure-orbital-ground-station-retirement
    // "orbitalresources",
    "patchassessmentresources",
    "patchinstallationresources",
    "policyresources",
    "recoveryservicesresources",
    "resourcechanges",
    "resourcecontainerchanges",
    "resourcecontainers",
    "resources",
    "securityresources",
    "servicehealthresources",
    "spotresources",
];

#[derive(Serialize, Deserialize, Debug, Clone)]
pub(crate) struct ResourceGraphRequest {
    subscriptions: Vec<String>,
    query: String,
    //  #[serde(skip_serializing_if = "Option::is_none")]
    options: ResourceGraphRequestOptions,
}

impl ResourceGraphRequest {
    pub(crate) fn new(subscription_id: &str, query: &str) -> Self {
        Self {
            subscriptions: vec![subscription_id.to_string()],
            query: query.to_string(),
            options: ResourceGraphRequestOptions {
                skip: None,
                skip_token: None,
                top: Some(1000),
                allow_partial_scopes: None,
            },
        }
    }

    fn add_skip_token(&mut self, skip_token: &str) {
        self.options.skip_token = Some(skip_token.to_string());
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ResourceGraphRequestOptions {
    #[serde(rename = "$skip")]
    #[serde(skip_serializing_if = "Option::is_none")]
    skip: Option<usize>,
    #[serde(rename = "$skipToken")]
    #[serde(skip_serializing_if = "Option::is_none")]
    skip_token: Option<String>,
    #[serde(rename = "$top")]
    #[serde(skip_serializing_if = "Option::is_none")]
    top: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    allow_partial_scopes: Option<bool>,
    // authorization_scope_filter: ...,
    // result_format: ...,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct QueryResponse {
    #[serde(rename = "$skipToken")]
    skip_token: Option<String>,
    count: usize,
    data: ResourceGraphData,
    facets: Value,
    result_truncated: String,
    total_records: usize,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct QueryError {
    error: QueryErrorError,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct QueryErrorError {
    code: QueryErrorErrorCode,
    details: Vec<QueryErrorErrorDetails>,
    message: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[non_exhaustive]
enum QueryErrorErrorCode {
    RateLimiting,
    BadRequest,
    Other(Value),
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct QueryErrorErrorDetails {
    code: QueryErrorErrorDetailsCode,
    message: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
enum QueryErrorErrorDetailsCode {
    RateLimiting,
    ResponsePayloadTooLarge,
    DisallowedLogicalTableName,
    Other(Value),
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
enum ResourceGraphResponse {
    Query(QueryResponse),
    Error(QueryError),
    Other(Value),
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(transparent)]
pub(crate) struct ResourceGraphData {
    inner: Vec<ResourceGraphDataInner>,
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct ResourceGraphDataInner {
    // Pull `type` out to make sure it's the first field in the
    // serialized output to workaround Splunk KV extraction limits
    r#type: String,
    #[serde(flatten)]
    other: HashMap<String, Value>,
}

impl ToHecEvents for &ResourceGraphData {
    type Item = ResourceGraphDataInner;
    fn source(&self) -> &str {
        "azure_resource_graph"
    }

    fn sourcetype(&self) -> &str {
        "azure_resource_graph"
    }
    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        Box::new(self.inner.iter())
    }
    fn ssphp_run_key(&self) -> &str {
        "azure_resource_graph"
    }
}

#[derive(Debug, Default)]
struct RateLimit {
    requests: VecDeque<Instant>,
    max_requests: usize,
    interval: Duration,
}

impl RateLimit {
    fn default() -> Self {
        Self {
            requests: VecDeque::new(),
            max_requests: 14,
            interval: Duration::from_millis(5100),
        }
    }

    async fn wait(&mut self) -> Result<()> {
        self.requests.push_back(Instant::now());
        if self.requests.len() > self.max_requests {
            let oldest = self
                .requests
                .pop_front()
                .expect("Checked len() for elements");
            let deadline = oldest
                .checked_add(self.interval)
                .expect("time to add correctly");
            tokio::time::sleep_until(deadline).await;
        }
        Ok(())
    }
}

#[cfg(feature = "live_tests")]
#[cfg(test)]
#[tokio::test]
async fn test_azure_resource_graph() -> Result<()> {
    let (azure_rest, splunk) = crate::azure_rest::live_tests::setup().await?;
    resource_graph_all(azure_rest, &splunk).await?;
    Ok(())
}
