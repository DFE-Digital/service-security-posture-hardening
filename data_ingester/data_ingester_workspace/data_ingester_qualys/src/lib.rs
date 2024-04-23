mod qvs;

use anyhow::{Context, Result};
use qvs::Qvs;
use reqwest::{
    header::{HeaderMap, HeaderValue},
    Client, Method, RequestBuilder,
};
use tokio::time::{sleep, Duration};
use tracing::{debug, info, warn};

/// A simple Qualys client
#[derive(Debug, Default)]
pub struct Qualys {
    client: Client,
    username: String,
    password: String,
    limits: QualysLimits,
}

/// Limits to use when throttling Qualys requests
#[derive(Debug)]
struct QualysLimits {
    rate_limit: usize,
    rate_window_seconds: usize,
    rate_remaining: usize,
    rate_to_wait_seconds: usize,
    concurrency_limit: usize,
    concurrency_running: usize,
}

impl Default for QualysLimits {
    /// Express/Consultant
    /// API Service Concurrency Limit per Subscription (per API): 1 call
    /// Rate Limit per Subscription (per API): 50 calls per Day
    ///
    /// Standard API
    /// Service Concurrency Limit per Subscription (per API): 2 calls
    /// Rate Limit per Subscription (per API): 300 calls per Hour
    ///
    /// Enterprise API Service
    /// Concurrency Limit per Subscription (per API): 5 calls
    /// Rate Limit per Subscription (per API): 750 calls per Hour
    ///
    /// Premium API Service
    /// Concurrency Limit per Subscription (per API): 10 calls
    /// Rate Limit per Subscription (per API): 2000 calls per Hour
    ///
    /// https://cdn2.qualys.com/docs/qualys-api-limits.pdf
    ///
    fn default() -> Self {
        Self {
            rate_limit: 300,
            rate_window_seconds: 60 * 60,
            rate_remaining: 300,
            rate_to_wait_seconds: 0,
            concurrency_limit: 2,
            concurrency_running: 0,
        }
    }
}

impl QualysLimits {
    /// Extract a limit header or provide a default value
    /// TODO check default value is sane
    fn get_usize_from_header(headers: &HeaderMap, key: &str) -> usize {
        static DEFAULT: usize = 0;
        headers
            .get(key)
            .map(|h| {
                h.to_str()
                    .unwrap_or_default()
                    .parse::<usize>()
                    .unwrap_or(DEFAULT)
            })
            .unwrap_or(DEFAULT)
    }

    /// Extract limit headers from a [reqwest::HeaderMap]
    pub(crate) fn from_headers(headers: &HeaderMap) -> Self {
        debug!("Qualys response headers: {:?}", headers);
        let limits = Self {
            rate_limit: QualysLimits::get_usize_from_header(headers, "X-RateLimit-Limit"),
            rate_window_seconds: QualysLimits::get_usize_from_header(
                headers,
                "X-RateLimit-Window-Sec",
            ),
            rate_remaining: QualysLimits::get_usize_from_header(headers, "X-RateLimit-Remaining"),
            rate_to_wait_seconds: QualysLimits::get_usize_from_header(
                headers,
                "X-RateLimit-ToWait-Sec",
            ),
            concurrency_limit: QualysLimits::get_usize_from_header(
                headers,
                "X-Concurrency-Limit-Limit",
            ),
            concurrency_running: QualysLimits::get_usize_from_header(
                headers,
                "X-Concurrency-Limit-Running",
            ),
        };
        debug!("Qualys parsed limits: {:?}", limits);
        limits
    }

    /// Wait for the rate limit to expire
    async fn wait(&self) {
        if self.rate_remaining > 1 && self.rate_to_wait_seconds > 0 {
            sleep(Duration::from_secs(self.rate_to_wait_seconds as u64)).await;
        }
    }
}

impl Qualys {
    /// Create a new Qualys client using basic auth
    pub fn new(username: &str, password: &str) -> Result<Self> {
        let client = reqwest::ClientBuilder::new()
            .default_headers(Qualys::headers().context("Building Qualys headers")?)
            .build()
            .context("Building Qualys reqwest client")?;
        info!("Qualys client: {:?}", client);
        Ok(Self {
            client,
            username: username.to_string(),
            password: password.to_string(),
            limits: QualysLimits::default(),
        })
    }

    /// RequestBuilder utilising basic_auth
    fn request(&self, method: Method, url: &str) -> RequestBuilder {
        self.client
            .request(method, url)
            .basic_auth(&self.username, Some(&self.password))
    }

    /// Default headers
    fn headers() -> Result<HeaderMap> {
        let mut headers = HeaderMap::new();

        let user_agent = HeaderValue::from_str("curl/8.4.0")?;
        _ = headers.insert("User-Agent", user_agent);

        let user_agent = HeaderValue::from_str("curl/8.4.0")?;
        _ = headers.insert("X-Requested-With", user_agent);

        let content_type = HeaderValue::from_str("application/json")?;
        _ = headers.insert("Content-Type", content_type);
        Ok(headers)
    }

    /// Get the Qvs data for a slice of CVE IDs
    ///
    /// cves:
    ///
    /// A list of CVE IDs to requset the data for e.g
    /// &["CVE-2021-36765"]
    pub async fn get_qvs(&mut self, cves: &[String]) -> Result<Qvs> {
        info!("Getting QVS data for {} CVEs", cves.len());
        let mut qvs = Qvs::default();
        // 450 comes from
        // https://github.com/buddybergman/Qualys-Get_QVS_Data/blob/e86fb599b783b871c8fbc1bc2fc1cadd9ec14b08/Get_Qualys_QVS_Details.py#L26
        for chunk in cves.chunks(450) {
            let cve = chunk.join(",").to_string();
            let url = format!("https://qualysapi.qg2.apps.qualys.eu/api/2.0/fo/knowledge_base/qvs/?action=list&details=All&cve={}", cve);
            let response = match self.request(Method::GET, &url).send().await {
                Ok(r) => r,
                Err(e) => {
                    warn!("Error while getting Qualys QVS data: {:?}", e);
                    continue;
                }
            };

            // TODO: Use limits to throttle requests
            self.limits = QualysLimits::from_headers(response.headers());
            info!("Qualys limits: {:?}", self.limits);
            self.limits.wait().await;

            let response_text = response.text().await?;
            let qvs_ = match serde_json::from_str::<Qvs>(&response_text) {
                Ok(qvs) => qvs,
                Err(e) => {
                    warn!(
                        "Error while deserializing Qualys QVS data: {:?},\nResponse body: {}",
                        e,
                        &response_text
                    );

                    anyhow::bail!("Failed deserializing Qvs JSON");
                }
            };
            qvs.extend(qvs_);
        }
        Ok(qvs)
    }
}
