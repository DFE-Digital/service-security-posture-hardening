use anyhow::{Context, Result};
use azure_core::auth::TokenCredential;
use data_ingester_splunk::splunk::{get_ssphp_run, hec_stats, SplunkTrait};
use data_ingester_splunk::splunk::{set_ssphp_run, Splunk, ToHecEvents};
use data_ingester_supporting::keyvault::Secrets;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::{HashMap, VecDeque},
    sync::Arc,
};
use tokio::time::{Duration, Instant};
use tracing::{error, info, warn};
use valuable::Valuable;

pub async fn azure_resource_graph(secrets: Arc<Secrets>, splunk: Arc<Splunk>) -> Result<()> {
    let _ = set_ssphp_run(crate::SSPHP_RUN_KEY)?;

    info!(
        name = crate::SSPHP_RUN_KEY,
        ssphp_run = get_ssphp_run(crate::SSPHP_RUN_KEY),
        git_build_hash = env!("GIT_HASH"),
        stage = "Starting"
    );

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
            let mut batch = 0;

            info!(
                name = crate::SSPHP_RUN_KEY,
                ssphp_run = get_ssphp_run(crate::SSPHP_RUN_KEY),
                subscription_id = sub_id,
                table = table,
                batch = batch,
                "Getting table for subscription"
            );

            let mut request_body =
                ResourceGraphRequest::new(sub_id, &format!("{} | order by name asc", &table));

            if *table == "guestconfigurationresources" {
                request_body.options.top = Some(10);
            }

            let mut response =
                match make_request(&az_client, endpoint, &request_body, &mut rate_limit).await {
                    Ok(response) => response,
                    Err(err) => {
                        error!(
                            name=crate::SSPHP_RUN_KEY,
                            ssphp_run=get_ssphp_run(crate::SSPHP_RUN_KEY),
                            subscription_id=sub_id,
                            table=table,
                            batch = batch,                            
                            error=?err,
                            "Failed making request for Azure resource graph table");
                        continue;
                    }
                };

            response.data.source = Some(format!("{}:{}:{}", sub_id, table, batch));

            let events = (&response.data)
                .to_hec_events()
                .context("Serialize ResourceGraphResponse.data events")?;

            let stats = hec_stats(&events);
            splunk
                .send_batch(events)
                .await
                .context("Sending events to Splunk")?;

            info!(
                name = crate::SSPHP_RUN_KEY,
                ssphp_run = get_ssphp_run(crate::SSPHP_RUN_KEY),
                subscription_id = sub_id,
                table = table,
                batch = batch,
                stats = &stats.as_value(),
                "Sent HecEvents to Splunk"
            );

            while let Some(ref skip_token) = response.skip_token {
                batch += 1;

                info!(
                    name = crate::SSPHP_RUN_KEY,
                    ssphp_run = get_ssphp_run(crate::SSPHP_RUN_KEY),
                    subscription_id = sub_id,
                    table = table,
                    batch = batch,
                    "Getting additional batches for table for subscription"
                );

                request_body.add_skip_token(skip_token);

                response = make_request(&az_client, endpoint, &request_body, &mut rate_limit)
                    .await
                    .context("Failed making Resource Graph API request")?;

                response.data.source = Some(format!("{}:{}:{}", sub_id, table, batch));

                let events = (&response.data)
                    .to_hec_events()
                    .context("Serialize ResourceGraphResponse.data events")?;

                let stats = hec_stats(&events);

                splunk
                    .send_batch(events)
                    .await
                    .context("Sending events to Splunk")?;

                info!(
                    name = crate::SSPHP_RUN_KEY,
                    ssphp_run = get_ssphp_run(crate::SSPHP_RUN_KEY),
                    subscription_id = sub_id,
                    table = table,
                    batch = batch,
                    stats = &stats.as_value(),
                    "Sent HecEvents to Splunk"
                );
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

//#[async_recursion]
async fn make_request(
    az_client: &AzureRest,
    endpoint: &str,
    request_body: &ResourceGraphRequest,
    rate_limit: &mut RateLimit,
) -> Result<QueryResponse> {
    let mut request_body = request_body.clone();
    let response = 'request: loop {
        rate_limit.wait().await?;

        let result = az_client
            .post_rest_request(endpoint, &request_body)
            .await
            .context("Sending Resource Graph Post Request")?;

        match result {
            // Happy path
            ResourceGraphResponse::Query(response) => break response,

            // Known errors
            ResourceGraphResponse::Error(ref error) => {
                match &error.error.code {
                    QueryErrorErrorCode::RateLimiting => {
                        error!("Rate limited!:\n {:?}", error);
                        tokio::time::sleep(rate_limit.interval).await;
                        continue;
                    }
                    QueryErrorErrorCode::BadRequest => {
                        let details = if let Some(details) = error.error.details.as_ref() {
                            details
                        } else {
                            error!(
                                name=crate::SSPHP_RUN_KEY,
                                ssphp_run=get_ssphp_run(crate::SSPHP_RUN_KEY),
                                request_body=request_body.as_value(),
                                response=?&result, "Unknown BadRequest Type");
                            anyhow::bail!("Unknown BadRequest Error Type : {:?}", result);
                        };

                        // The next for loop only iterates once, on
                        // the first entry in details.  How often do
                        // we get more than one error details? Should
                        // this observe all errors, then decide on an
                        // action?
                        if details.len() > 1 {
                            warn!( details=?details, details_len=details.len(), "");
                        }
                        #[allow(clippy::never_loop)]
                        for bad_request_error in details {
                            match &bad_request_error.code {
                                QueryErrorErrorDetailsCode::ResponsePayloadTooLarge => {
                                    error!(
                                        name = crate::SSPHP_RUN_KEY,
                                        ssphp_run = get_ssphp_run(crate::SSPHP_RUN_KEY),
                                        error = error.as_value(),
                                        request_body = request_body.as_value(),
                                        "ResponsePayloadTooLarge error!"
                                    );
                                    request_body.options.top = request_body
                                        .options
                                        .top
                                        .map(|top| std::cmp::max(top / 2, 1))
                                        .or(Some(500));
                                    continue 'request;
                                }

                                QueryErrorErrorDetailsCode::RateLimiting => {
                                    error!(
                                        name = crate::SSPHP_RUN_KEY,
                                        ssphp_run = get_ssphp_run(crate::SSPHP_RUN_KEY),
                                        error = error.as_value(),
                                        request_body = request_body.as_value(),
                                        "Rate limited",
                                    );
                                    tokio::time::sleep(rate_limit.interval).await;
                                    continue 'request;
                                }

                                QueryErrorErrorDetailsCode::DisallowedLogicalTableName => {
                                    error!(
                                        name = crate::SSPHP_RUN_KEY,
                                        ssphp_run = get_ssphp_run(crate::SSPHP_RUN_KEY),
                                        error = error.as_value(),
                                        request_body = request_body.as_value(),
                                        "Disallowed Logical Table"
                                    );
                                    anyhow::bail!("Disallowed Logical Table: {:?}", &request_body);
                                }

                                // Unknown Errors and responses
                                QueryErrorErrorDetailsCode::Other(other) => {
                                    error!(
                                        name = crate::SSPHP_RUN_KEY,
                                        ssphp_run = get_ssphp_run(crate::SSPHP_RUN_KEY),
                                        error = error.as_value(),
                                        request_body = request_body.as_value(),
                                        "Unknown QueryErrorErrorDetailsCode"
                                    );
                                    anyhow::bail!("Unknown Error Type : {:?}", other);
                                }
                            }
                        }
                    }
                    // Unknown Errors and responses
                    QueryErrorErrorCode::Other(other) => {
                        error!(
                            name = crate::SSPHP_RUN_KEY,
                            ssphp_run = get_ssphp_run(crate::SSPHP_RUN_KEY),
                            error = error.as_value(),
                            request_body = request_body.as_value(),
                            "Unknown QueryErrorErrorCode"
                        );
                        anyhow::bail!("Unknown Error Type : {:?}", other);
                    }
                }
            }
            // Unknown Errors and responses
            ResourceGraphResponse::Other(other) => {
                error!(
                    name=crate::SSPHP_RUN_KEY,
                    ssphp_run=get_ssphp_run(crate::SSPHP_RUN_KEY),
                    // TODO: Serialize serde_json::Value as Valuable.
                    error=?other,
                    request_body=request_body.as_value(),
                    "Unknown error response from Azure Resource Graph");
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

#[derive(Valuable, Serialize, Deserialize, Debug, Clone)]
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

#[derive(Valuable, Serialize, Deserialize, Debug, Clone)]
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

/// https://learn.microsoft.com/en-us/graph/errors#json-representation
#[derive(Valuable, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct QueryError {
    error: QueryErrorError,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
enum ResourceGraphResponse {
    Query(QueryResponse),
    Error(QueryError),
    Other(Value),
}

#[test]
fn test_json_into_resource_graph_response_error() {
    let error_response = r#"
{
  "error": {
    "code": "BadRequest",
    "details": [
      {
        "code": "ResponsePayloadTooLarge",
        "message": "Response payload size is ..."
      }
    ],
    "message": "Please provide below info when asking for support"
  }
}"#;
    // let obj: QueryError =
    //     serde_json::from_str(&error_response).expect("JSON should parse into QueryError");
    let obj: ResourceGraphResponse =
        serde_json::from_str(error_response).expect("JSON should parse into ResoureGraphResponse");
    assert!(
        matches!(obj, ResourceGraphResponse::Error(_)),
        "JSON didn't parse into a ResourceGraphResponse::Error"
    );
}

/// https://learn.microsoft.com/en-us/graph/errors#json-representation
#[derive(Valuable, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct QueryErrorError {
    /// An error code string for the error that occurred
    code: QueryErrorErrorCode,

    /// A developer ready message about the error that occurred. This shouldn't be displayed to the user directly.
    message: String,

    /// Optional. A list of more error objects that might provide a breakdown of multiple errors encountered while processing the request.
    details: Option<Vec<QueryErrorErrorDetails>>,

    /// Optional. An additional error object that might be more specific than the top-level error.
    inner_error: Option<QueryErrorInnerError>,
}

#[derive(Valuable, Serialize, Deserialize, Debug)]
struct QueryErrorInnerError {
    /// An error code string for the error that occurred
    code: QueryErrorErrorCode,
    /// Optional. A list of more error objects that might provide a breakdown of multiple errors encountered while processing the request.
    details: Vec<QueryErrorErrorDetails>,
    /// A developer ready message about the error that occurred. This shouldn't be displayed to the user directly.
    message: String,
}

#[derive(Valuable, Serialize, Deserialize, Debug)]
#[non_exhaustive]
enum QueryErrorErrorCode {
    RateLimiting,
    BadRequest,
    Other(#[valuable(skip)] Value),
}

#[derive(Valuable, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct QueryErrorErrorDetails {
    code: QueryErrorErrorDetailsCode,
    message: String,
}

#[derive(Valuable, Serialize, Deserialize, Debug)]
enum QueryErrorErrorDetailsCode {
    RateLimiting,
    ResponsePayloadTooLarge,
    DisallowedLogicalTableName,
    Other(#[valuable(skip)] Value),
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(transparent)]
pub(crate) struct ResourceGraphData {
    inner: Vec<ResourceGraphDataInner>,
    #[serde(default, skip)]
    source: Option<String>,
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
        self.source.as_deref().unwrap_or("NO_SOURCE_SET")
    }

    fn sourcetype(&self) -> &str {
        "azure_resource_graph"
    }
    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        Box::new(self.inner.iter())
    }
    fn ssphp_run_key(&self) -> &str {
        crate::SSPHP_RUN_KEY
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
