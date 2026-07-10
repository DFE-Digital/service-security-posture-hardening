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
            let mut adaptive_top = AdaptiveTop::initial_for_table(table);

            info!(
                name = crate::SSPHP_RUN_KEY,
                ssphp_run = get_ssphp_run(crate::SSPHP_RUN_KEY),
                subscription_id = sub_id,
                table = table,
                batch = batch,
                top = adaptive_top.value(),
                "Getting table for subscription"
            );

            let mut request_body =
                ResourceGraphRequest::new(sub_id, &format!("{} | order by name asc", table));

            let mut response = match make_request(
                &az_client,
                endpoint,
                &request_body,
                &mut rate_limit,
                &mut adaptive_top,
            )
            .await
            {
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
                    top = adaptive_top.value(),
                    "Getting additional batches for table for subscription"
                );

                request_body.add_skip_token(skip_token);

                response = make_request(
                    &az_client,
                    endpoint,
                    &request_body,
                    &mut rate_limit,
                    &mut adaptive_top,
                )
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
    adaptive_top: &mut AdaptiveTop,
) -> Result<QueryResponse> {
    let mut request_body = request_body.clone();
    let response = 'request: loop {
        adaptive_top.apply_to(&mut request_body);
        rate_limit.wait().await?;

        let result = az_client
            .post_rest_request(endpoint, &request_body)
            .await
            .context("Sending Resource Graph Post Request")?;

        match result {
            // Happy path
            ResourceGraphResponse::Query(response) => {
                let requested_top = request_body.options.top.unwrap_or(AdaptiveTop::MAX_TOP);
                let payload_bytes = response.data.serialized_payload_bytes();
                adaptive_top.tune_from_response(payload_bytes, response.count, requested_top);
                info!(
                    name = crate::SSPHP_RUN_KEY,
                    ssphp_run = get_ssphp_run(crate::SSPHP_RUN_KEY),
                    requested_top = requested_top,
                    next_top = adaptive_top.value(),
                    records = response.count,
                    payload_bytes = payload_bytes,
                    "Adaptive top tuned from response"
                );
                break response;
            }

            // Known errors
            ResourceGraphResponse::Error(ref error) => {
                match &error.error.code {
                    QueryErrorErrorCode::RateLimiting => {
                        error!("Rate limited!:\n {:?}", error);
                        tokio::time::sleep(rate_limit.interval).await;
                        continue;
                    }
                    QueryErrorErrorCode::GatewayTimeout => {
                        error!(
                            name = crate::SSPHP_RUN_KEY,
                            ssphp_run = get_ssphp_run(crate::SSPHP_RUN_KEY),
                            error = error.as_value(),
                            request_body = request_body.as_value(),
                            "GatewayTimeout error!"
                        );
                        adaptive_top.shrink_for_error();
                        tokio::time::sleep(rate_limit.interval).await;
                        continue 'request;
                    }
                    QueryErrorErrorCode::InternalServerError => {
                        error!(
                            name = crate::SSPHP_RUN_KEY,
                            ssphp_run = get_ssphp_run(crate::SSPHP_RUN_KEY),
                            error = error.as_value(),
                            request_body = request_body.as_value(),
                            "InternalServerError from Azure Resource Graph, retrying"
                        );
                        adaptive_top.shrink_for_error();
                        tokio::time::sleep(rate_limit.interval).await;
                        continue 'request;
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
                                    adaptive_top.shrink_for_error();
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
                                    anyhow::bail!("Disallowed Logical Table: {:?}", request_body);
                                }

                                QueryErrorErrorDetailsCode::UnexpectedQueryExecutionError => {
                                    error!(
                                        name = crate::SSPHP_RUN_KEY,
                                        ssphp_run = get_ssphp_run(crate::SSPHP_RUN_KEY),
                                        error = error.as_value(),
                                        request_body = request_body.as_value(),
                                        "UnexpectedQueryExecutionError from Azure Resource Graph, retrying"
                                    );
                                    adaptive_top.shrink_for_error();
                                    tokio::time::sleep(rate_limit.interval).await;
                                    continue 'request;
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
fn test_json_into_resource_graph_response_gateway_timeout() {
    let error_response = r#"
{
  "error": {
    "code": "GatewayTimeout",
    "message": "The gateway did not receive a response from 'Microsoft.ResourceGraph' within the specified time period."
  }
}"#;
    let obj: ResourceGraphResponse =
        serde_json::from_str(error_response).expect("JSON should parse into ResourceGraphResponse");
    assert!(
        matches!(
            obj,
            ResourceGraphResponse::Error(QueryError {
                error: QueryErrorError {
                    code: QueryErrorErrorCode::GatewayTimeout,
                    ..
                }
            })
        ),
        "JSON didn't parse into a ResourceGraphResponse::Error(GatewayTimeout)"
    );
}

#[test]
fn test_json_into_resource_graph_response_internal_server_error() {
    let error_response = r#"
{
  "error": {
    "code": "InternalServerError",
    "message": "Please provide below info when asking for support: timestamp = 2026-07-10T08:32:11.4933442Z, correlationId = 8dd19e36-e51b-4ae2-ae80-eddd6d840406.",
    "details": [
      {
        "code": "UnexpectedQueryExecutionError",
        "message": "An unexpected query execution error occurred. Please try again later."
      }
    ]
  }
}"#;
    let obj: ResourceGraphResponse =
        serde_json::from_str(error_response).expect("JSON should parse into ResourceGraphResponse");
    assert!(
        matches!(
            obj,
            ResourceGraphResponse::Error(QueryError {
                error: QueryErrorError {
                    code: QueryErrorErrorCode::InternalServerError,
                    ..
                }
            })
        ),
        "JSON didn't parse into a ResourceGraphResponse::Error(InternalServerError)"
    );
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
    GatewayTimeout,
    InternalServerError,
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
    UnexpectedQueryExecutionError,
    Other(#[valuable(skip)] Value),
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(transparent)]
pub(crate) struct ResourceGraphData {
    inner: Vec<ResourceGraphDataInner>,
    #[serde(default, skip)]
    source: Option<String>,
}

impl ResourceGraphData {
    fn serialized_payload_bytes(&self) -> usize {
        serde_json::to_string(&self.inner)
            .map(|json| json.len())
            .unwrap_or(0)
    }
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

#[derive(Debug)]
struct AdaptiveTop {
    current: usize,
    last_bytes_per_record: Option<usize>,
}

impl AdaptiveTop {
    /// Azure Resource Graph enforces a ~16 MiB response limit.
    const RESPONSE_BUDGET_BYTES: usize = 12 * 1024 * 1024;
    /// Conservative per-record ceiling used before any successful page is observed.
    const WORST_CASE_RECORD_BYTES: usize = 3 * 1024 * 1024;
    const MAX_TOP: usize = 1000;
    const MIN_TOP: usize = 1;

    fn initial_for_table(table: &str) -> Self {
        Self {
            current: match table {
                "guestconfigurationresources" => 10,
                // Large, variable rows — start conservative and let tuning ramp up.
                "securityresources" => 50,
                _ => Self::MAX_TOP,
            },
            last_bytes_per_record: None,
        }
    }

    fn value(&self) -> usize {
        self.current
    }

    fn apply_to(&self, request: &mut ResourceGraphRequest) {
        request.options.top = Some(self.current);
    }

    fn shrink_for_error(&mut self) {
        let bytes_per_record = self
            .last_bytes_per_record
            .unwrap_or(Self::WORST_CASE_RECORD_BYTES);
        let safe_top = (Self::RESPONSE_BUDGET_BYTES * 9 / 10 / bytes_per_record)
            .clamp(Self::MIN_TOP, Self::MAX_TOP);

        if self.current > safe_top {
            self.current = safe_top;
        } else {
            self.current = (self.current / 2).max(Self::MIN_TOP);
        }
    }

    fn tune_from_response(
        &mut self,
        payload_bytes: usize,
        records_returned: usize,
        requested_top: usize,
    ) {
        if records_returned == 0 {
            return;
        }

        let bytes_per_record = payload_bytes.div_ceil(records_returned).max(1);
        self.last_bytes_per_record = Some(bytes_per_record);

        let ideal_top =
            (Self::RESPONSE_BUDGET_BYTES / bytes_per_record).clamp(Self::MIN_TOP, Self::MAX_TOP);

        if records_returned < requested_top {
            // Final page for this query — keep the current top for any follow-up work.
            return;
        }

        if payload_bytes < Self::RESPONSE_BUDGET_BYTES / 2 {
            self.current = ((self.current * 2 + ideal_top) / 3).clamp(Self::MIN_TOP, Self::MAX_TOP);
        } else if payload_bytes > Self::RESPONSE_BUDGET_BYTES {
            self.current = ideal_top.min(self.current);
        } else {
            self.current = ((self.current + ideal_top) / 2).clamp(Self::MIN_TOP, Self::MAX_TOP);
        }
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

#[cfg(test)]
mod adaptive_top_tests {
    use super::AdaptiveTop;

    #[test]
    fn shrink_without_history_uses_worst_case_record_size() {
        let mut top = AdaptiveTop::initial_for_table("securityresources");
        assert_eq!(top.value(), 50);

        top.shrink_for_error();

        // 12 MiB * 0.9 / 3 MiB ≈ 3 records per page
        assert_eq!(top.value(), 3);
    }

    #[test]
    fn shrink_with_observed_record_size_targets_safe_top() {
        let mut top = AdaptiveTop::initial_for_table("resources");
        top.tune_from_response(10 * 1024 * 1024, 20, 1000);

        top.shrink_for_error();

        // 12 MiB * 0.9 / 512 KiB ≈ 21 records per page
        assert_eq!(top.value(), 21);
    }

    #[test]
    fn shrink_halves_when_already_below_safe_top() {
        let mut top = AdaptiveTop::initial_for_table("securityresources");
        top.tune_from_response(10 * 1024 * 1024, 100, 100);

        top.shrink_for_error();

        assert_eq!(top.value(), 42);
    }

    #[test]
    fn tune_grows_when_pages_are_small() {
        let mut top = AdaptiveTop::initial_for_table("securityresources");
        top.tune_from_response(1 * 1024 * 1024, 50, 50);

        assert!(top.value() > 50);
        assert!(top.value() <= AdaptiveTop::MAX_TOP);
    }

    #[test]
    fn tune_shrinks_when_page_exceeds_budget() {
        let mut top = AdaptiveTop::initial_for_table("resources");
        top.tune_from_response(14 * 1024 * 1024, 1000, 1000);

        assert!(top.value() < 1000);
    }

    #[test]
    fn tune_does_not_grow_on_final_partial_page() {
        let mut top = AdaptiveTop::initial_for_table("securityresources");
        top.tune_from_response(512 * 1024, 7, 50);

        assert_eq!(top.value(), 50);
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
