use anyhow::{Context, Result};
use reqwest::{
    header::{HeaderMap, HeaderValue},
    Client,
};
use serde::{de::DeserializeOwned, Deserialize};
use tracing::info;

pub struct SplunkApiClient {
    /// A reqwest client
    client: Client,
    /// The full URL to splunk including protocol and port
    url_base: String,
    /// The Splunk application to route searches to
    app: String,
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
    /// Setting an envionment variable called "ACCEPT_INVALID_CERTS"
    /// to any value will disable certificate checking. This can be
    /// used when connecting to green Splunk Docker instances.
    ///
    pub fn new(url_base: &str, token: &str) -> Result<Self> {
        let client = if std::env::var_os("ACCEPT_INVALID_CERTS").is_none() {
            reqwest::ClientBuilder::new()
                .danger_accept_invalid_certs(false)
                .default_headers(SplunkApiClient::headers(token)?)
                .build()?
        } else {
            reqwest::ClientBuilder::new()
                .danger_accept_invalid_certs(true)
                .default_headers(SplunkApiClient::headers(token)?)
                .build()?
        };
        Ok(Self {
            client,
            url_base: url_base.to_owned(),
            app: "search".to_string(),
        })
    }

    /// Set a different
    ///
    /// The standard app is 'search' use this method to set a
    /// different Splunk app for searches
    pub fn set_app(mut self, app: &str) -> Self {
        self.app = app.to_string();
        self
    }

    fn headers(token: &str) -> Result<HeaderMap> {
        let mut headers = HeaderMap::new();
        let mut auth = HeaderValue::from_str(&format!("Splunk {}", token))?;
        auth.set_sensitive(true);
        _ = headers.insert("Authorization", auth);
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
        let form = [
            ("search", search),
            ("output_mode", "json"),
            ("exec_mode", "oneshot"),
        ];
        let result = self
            .client
            .post(&self.search_url())
            .form(&form)
            .send()
            .await
            .context("Sending search request")?
            .text()
            .await
            .context("Getting search response body")?
            .lines()
//            .inspect(|line| info!("line: {:?}", line))
            .flat_map(serde_json::from_str::<SearchResult<T>>)
            .map(|sr| sr.result)
            .collect();

        Ok(result)
    }

    /// The full url endpoint for Splunk searches
    fn search_url(&self) -> String {
        format!(
            "{}/servicesNS/nobody/{}/search/v2/jobs/export",
            self.url_base, self.app
        )
    }
}

#[cfg(test)]
mod test {
    use super::SplunkApiClient;
    use anyhow::Result;
    use serde::Deserialize;

    #[allow(dead_code)]
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
        dbg!(results.iter().take(2).collect::<Vec<&TestSplunkResults>>());
        assert!(!results.is_empty());
        Ok(())
    }

    #[test]
    fn test_splunk_search_url() -> Result<()> {
        let client =
            SplunkApiClient::new("https://foo.splunkcloud.com:8089", "bar")?.set_app("custom_app");
        let url = client.search_url();
        let expected =
            "https://foo.splunkcloud.com:8089/servicesNS/nobody/custom_app/search/v2/jobs/export";
        assert_eq!(expected, url);
        Ok(())
    }
}
