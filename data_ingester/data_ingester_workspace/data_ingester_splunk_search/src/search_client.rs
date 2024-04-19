use anyhow::{Context, Result};
use reqwest::{
    header::{HeaderMap, HeaderValue},
    Client,
};
//use tracing::info;

pub struct SplunkApiClient {
    client: Client,
    url_base: String,
}

impl SplunkApiClient {
    pub fn new(stack: &str, token: &str) -> Result<Self> {
        let client = reqwest::ClientBuilder::new()
            .danger_accept_invalid_certs(false)
            .default_headers(SplunkApiClient::headers(token)?)
            .build()?;
        let url_base = format!("https://{}.splunkcloud.com:8089", &stack);
        Ok(Self { client, url_base })
    }

    fn headers(token: &str) -> Result<HeaderMap> {
        let mut headers = HeaderMap::new();
        let mut auth = HeaderValue::from_str(&format!("Splunk {}", token))?;
        auth.set_sensitive(true);
        _ = headers.insert("Authorization", auth);
        // let content_type = HeaderValue::from_str("application/json")?;
        // _ = headers.insert("Content-Type", content_type);
        Ok(headers)
    }

    pub async fn run_search(&self, search_name: &str) -> Result<String> {
        let url = format!(
            "{}/servicesNS/nobody/SSPHP_metrics/search/v2/jobs/export",
            self.url_base
        );
        let form = [("search", search_name), ("output_mode", "raw")];
        let raw = self
            .client
            .post(&url)
            .form(&form)
            .send()
            .await
            .context("Sending search request")?
            .text()
            .await
            .context("Getting search response body")?;

        Ok(raw)
    }
}
