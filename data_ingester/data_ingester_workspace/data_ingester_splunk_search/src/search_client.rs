use anyhow::{Context, Result};
use reqwest::{
    header::{HeaderMap, HeaderValue},
    Client,
};
use serde::{de::DeserializeOwned, Deserialize};

pub struct SplunkApiClient {
    client: Client,
    url_base: String,
}

#[derive(Default, Debug, Clone, PartialEq, Deserialize)]
pub struct SearchResult<T> {
    // preview: bool,
    // offest: usize,
    result: T,
    // #[serde(default)]
    // last_row: bool
}

impl SplunkApiClient {
    /// Create a new Splunk API client
    ///
    /// url_base:
    /// The fully qualified url without a trailing slash example: https://foo.splunkcloud.com:8089
    ///
    /// token:
    /// A JWT token for Splunk access. This can be retrieved
    /// from 'Settings -> Token' in the Splunk console
    ///
    pub fn new(url_base: &str, token: &str) -> Result<Self> {
        let client = reqwest::ClientBuilder::new()
            .danger_accept_invalid_certs(false)
            .default_headers(SplunkApiClient::headers(token)?)
            .build()?;
        Ok(Self {
            client,
            url_base: url_base.to_owned(),
        })
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

    /// Run a splunk search against the clients Splunk instance
    ///
    /// search:
    /// The search to run e.g `| search index=foo | table _time sourcetype`
    ///
    /// T:
    /// The type to Deserialize into any missing fields will not be
    /// returned on generate errors
    ///
    /// Use serde_json::Value for unknown data
    pub async fn run_search<T: DeserializeOwned>(&self, search: &str) -> Result<Vec<T>> {
        let url = format!(
            "{}/servicesNS/nobody/search/search/v2/jobs/export",
            self.url_base
        );
        let form = [
            ("search", search),
            ("output_mode", "json"),
            ("exec_mode", "oneshot"),
        ];
        let result = self
            .client
            .post(&url)
            .form(&form)
            .send()
            .await
            .context("Sending search request")?
            .text()
            .await
            .context("Getting search response body")?
            .lines()
            .flat_map(serde_json::from_str::<SearchResult<T>>)
            .map(|sr| sr.result)
            .collect();

        Ok(result)
    }
}

#[cfg(test)]
mod test {
    use super::SplunkApiClient;
    use anyhow::Result;
    use serde::Deserialize;

    #[derive(Deserialize, Debug)]
    struct TestSplunkResults {
        _time: String,
        index: String,
    }

    #[ignore]
    #[tokio::test]
    async fn test_splunk_search() -> Result<()> {
        let client = SplunkApiClient::new(
            &std::env::var("splunk_rest_host").expect("Envionment variable"),
            &std::env::var("splunk_rest_token").expect("Envionment variable"),
        )?;
        let results = client
            .run_search::<TestSplunkResults>(
                "| search index=_* | table _time index source sourcetype",
            )
            .await?;
        dbg!(results.iter().take(2));
        assert!(!results.is_empty());
        Ok(())
    }
}
