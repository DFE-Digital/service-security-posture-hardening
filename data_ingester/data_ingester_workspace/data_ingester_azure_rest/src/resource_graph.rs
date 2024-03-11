use anyhow::{Context, Result};

use azure_core::auth::TokenCredential;
use data_ingester_splunk::splunk::{set_ssphp_run, Splunk, ToHecEvents};
use data_ingester_supporting::keyvault::Secrets;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;

use crate::azure_rest::AzureRest;
pub(crate) static RESOURCE_GRAPH_TABLES: [&str; 28] = [
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
    "orbitalresources",
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
    "spotresources ",
];

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct ResourceGraphRequest {
    subscriptions: Vec<String>,
    query: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    options: Option<ResourceGraphRequestOptions>,
}

impl ResourceGraphRequest {
    pub(crate) fn new(subscription_id: &str, query: &str) -> Self {
        Self {
            subscriptions: vec![subscription_id.to_string()],
            query: query.to_string(),
            options: Some(ResourceGraphRequestOptions {
                skip: None,
                skip_token: None,
                top: None,
                allow_partial_scopes: None,
            }),
        }
    }

    pub(crate) fn add_options(&mut self, options: ResourceGraphRequestOptions) {
        self.options = Some(options)
    }

    fn add_skip_token(&mut self, skip_token: &str) {
        if let Some(ref mut options) = self.options {
            options.skip_token = Some(skip_token.to_string());
        } else {
            self.options = Some(ResourceGraphRequestOptions {
                skip: None,
                skip_token: Some(skip_token.to_string()),
                top: None,
                allow_partial_scopes: None,
            })
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
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
pub(crate) struct ResourceGraphResponse {
    #[serde(rename = "$skipToken")]
    skip_token: Option<String>,
    count: usize,
    data: ResourceGraphData,
    facets: Value,
    result_truncated: String,
    total_records: usize,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(transparent)]
pub(crate) struct ResourceGraphData {
    inner: Vec<Value>,
}

impl ToHecEvents for &ResourceGraphData {
    type Item = Value;
    fn source(&self) -> &str {
        "azure_resource_graph_TBD"
    }

    fn sourcetype(&self) -> &str {
        "azure_resource_graph_TBD"
    }
    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        Box::new(self.inner.iter())
    }
}

async fn resource_graph_all(az_client: AzureRest, splunk: &Splunk) -> Result<()> {
    let endpoint = "https://management.azure.com/providers/Microsoft.ResourceGraph/resources?api-version=2021-03-01";

    // 15 requests per 5 seconds = 0.3 per sec
    let sleep = tokio::time::Duration::from_millis(400);
    for sub in az_client.subscriptions().inner.iter() {
        let sub_id = sub.subscription_id.as_ref().context("no subscription_id")?;
        dbg!(&sub_id);
        for table in &crate::resource_graph::RESOURCE_GRAPH_TABLES {
            dbg!(&table);
            let mut request_body =
                ResourceGraphRequest::new(sub_id, &format!("{} | order by name asc", &table));

            let mut response: crate::resource_graph::ResourceGraphResponse = az_client
                .post_rest_request(endpoint, &request_body)
                .await
                .context("Sending initial Resource Graph Request")?;

            let events = (&response.data)
                .to_hec_events()
                .context("Serialize ResourceGraphResponse.data events")?;
            splunk
                .send_batch(events)
                .await
                .context("Sending events to Splunk")?;

            tokio::time::sleep(sleep).await;

            while let Some(ref skip_token) = response.skip_token {
                dbg!("loop");
                request_body.add_skip_token(dbg!(skip_token));
                response = az_client
                    .post_rest_request(endpoint, &request_body)
                    .await
                    .context("Sending Resource Graph Request with skip_token")?;

                let events = (&response.data)
                    .to_hec_events()
                    .context("Serialize ResourceGraphResponse.data events")?;
                splunk
                    .send_batch(events)
                    .await
                    .context("Sending events to Splunk")?;
                tokio::time::sleep(sleep).await;
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

pub async fn azure_resource_graph(secrets: Arc<Secrets>, splunk: Arc<Splunk>) -> Result<()> {
    set_ssphp_run()?;

    splunk
        .log("Starting Azure Resource Graph collection")
        .await?;
    splunk
        .log(&format!("GIT_HASH: {}", env!("GIT_HASH")))
        .await
        .context("Failed logging to Splunk")?;

    let azure_rest = AzureRest::new(
        &secrets.azure_client_id,
        &secrets.azure_client_secret,
        &secrets.azure_tenant_id,
    )
    .await
    .context("Can't build rest client")?;
    resource_graph_all(azure_rest, &splunk)
        .await
        .context("Running azure_resource_graph")?;

    Ok(())
}

#[cfg(test)]
#[tokio::test]
async fn test_azure_resource_graph() -> Result<()> {
    let (azure_rest, splunk) = crate::azure_rest::test::setup().await?;
    resource_graph_all(azure_rest, &splunk).await?;
    assert!(false);
    Ok(())
}
