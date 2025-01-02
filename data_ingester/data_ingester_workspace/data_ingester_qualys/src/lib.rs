pub mod entrypoint;
mod limits;
mod qvs;
use anyhow::{Context, Result};
use limits::QualysLimits;
use qvs::Qvs;
use reqwest::{
    header::{HeaderMap, HeaderValue},
    Client, Method, RequestBuilder,
};
use tracing::{info, warn};

/// A simple Qualys client
#[derive(Debug, Default)]
pub struct Qualys {
    client: Client,
    username: String,
    password: String,
    limits: QualysLimits,
    host: String,
}

impl Qualys {
    /// Create a new Qualys client using basic auth
    pub fn new(username: &str, password: &str, host: Option<&str>) -> Result<Self> {
        let client = reqwest::ClientBuilder::new()
            .default_headers(Qualys::headers().context("Building Qualys headers")?)
            .build()
            .context("Building Qualys reqwest client")?;
        info!("Qualys client: {:?}", client);
        let host = if let Some(host) = host {
            host.to_string()
        } else {
            "https://qualysapi.qg2.apps.qualys.eu".to_string()
        };
        Ok(Self {
            client,
            username: username.to_string(),
            password: password.to_string(),
            limits: QualysLimits::default(),
            host,
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
            let mut retries = 5;
            'retry: loop {
                let cve = chunk.join(",").to_string();
                let url = format!(
                    "{}/api/2.0/fo/knowledge_base/qvs/?action=list&details=All&cve={}",
                    self.host, cve
                );
                info!(url=?url);
                let response = match self.request(Method::GET, &url).send().await {
                    Ok(r) => r,
                    Err(err) => {
                        warn!(error=?err, "Error while getting Qualys QVS data");
                        retries -= 1;
                        if retries > 0 {
                            continue 'retry;
                        } else {
                            break 'retry;
                        }
                    }
                };

                self.limits = QualysLimits::from_headers(response.headers());
                info!("Qualys limits: {:?}", self.limits);
                self.limits.wait().await;

                let response_text = response.text().await?;
                let qvs_ = match serde_json::from_str::<Qvs>(&response_text) {
                    Ok(qvs) => qvs,
                    Err(err) => {
                        warn!(error=?err, response_body=?response_text,
                            "Error while deserializing Qualys QVS data",
                        );
                        retries -= 1;
                        if retries > 0 {
                            continue 'retry;
                        } else {
                            anyhow::bail!("Failed deserializing Qvs JSON");
                        }
                    }
                };
                qvs.extend(qvs_);
                break 'retry;
            }
        }
        Ok(qvs)
    }
}

#[cfg(test)]
mod test {
    use mockito::{Server, ServerGuard};
    use tokio::time::Instant;
    use tracing::{info, subscriber::DefaultGuard};

    use crate::{qvs::Qvs, Qualys};

    async fn setup() -> (Qualys, ServerGuard, DefaultGuard) {
        // Start tracing
        let subscriber = tracing_subscriber::FmtSubscriber::new();
        let tracing_guard = tracing::subscriber::set_default(subscriber);

        // Start Mockito server
        let mock_server = Server::new_async().await;

        // Setup SendingTask
        let url = format!("http://{}", mock_server.host_with_port());

        let qualys = Qualys::new("username", "password", Some(&url)).unwrap();
        (qualys, mock_server, tracing_guard)
    }

    fn mock_response() -> Qvs {
        Qvs::default()
    }

    fn mock_response_body() -> String {
        serde_json::to_string(&mock_response()).expect("Serialization shouldn't fail")
    }

    #[tokio::test]
    async fn test_qualys_get_single_cve() {
        let (mut qualys, mut mock_server, _tracing_guard) = setup().await;

        let mock = mock_server
            .mock("GET", "/api/2.0/fo/knowledge_base/qvs/")
            .match_query("action=list&details=All&cve=TEST-CVE")
            .with_status(200)
            .with_body(mock_response_body())
            .create();

        let _ = qualys.get_qvs(&["TEST-CVE".to_string()]).await.unwrap();

        mock.assert();
    }

    #[tokio::test]
    async fn test_qualys_get_single_cve_failed_serialization_then_success() {
        let (mut qualys, mut mock_server, _tracing_guard) = setup().await;

        let mock_bad_response = mock_server
            .mock("GET", "/api/2.0/fo/knowledge_base/qvs/")
            .match_query("action=list&details=All&cve=TEST-CVE")
            .with_status(200)
            .with_body("BAD_BODY")
            .expect(1)
            .create();

        let mock_good_response = mock_server
            .mock("GET", "/api/2.0/fo/knowledge_base/qvs/")
            .match_query("action=list&details=All&cve=TEST-CVE")
            .with_status(200)
            .with_body(mock_response_body())
            .expect(1)
            .create();

        let _ = qualys.get_qvs(&["TEST-CVE".to_string()]).await.unwrap();

        mock_bad_response.assert();
        mock_good_response.assert();
    }

    #[tokio::test]
    async fn test_qualys_get_single_cve_failed_serialization() {
        let (mut qualys, mut mock_server, _tracing_guard) = setup().await;

        let mock_bad_response = mock_server
            .mock("GET", "/api/2.0/fo/knowledge_base/qvs/")
            .match_query("action=list&details=All&cve=TEST-CVE")
            .with_status(200)
            .with_body("BAD_BODY")
            .expect(5)
            .create();

        let result = qualys.get_qvs(&["TEST-CVE".to_string()]).await;

        mock_bad_response.assert();
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_qualys_get_multiple_batches() {
        let (mut qualys, mut mock_server, _tracing_guard) = setup().await;

        let mut cves: Vec<String> = (0..450).map(|x| x.to_string()).collect();
        info!(cves_len=?cves.len());
        let query = format!("action=list&details=All&cve={}", cves.join(","));

        let mock_first_batch = mock_server
            .mock("GET", "/api/2.0/fo/knowledge_base/qvs/")
            .match_query(query.as_str())
            .with_status(200)
            .with_body(mock_response_body())
            .expect(1)
            .create();

        let mock_second_batch = mock_server
            .mock("GET", "/api/2.0/fo/knowledge_base/qvs/")
            .match_query("action=list&details=All&cve=450")
            .with_status(200)
            .with_body(mock_response_body())
            .expect(1)
            .create();

        cves.push("450".to_string());

        let _ = qualys.get_qvs(&cves).await.unwrap();

        mock_first_batch.assert();
        mock_second_batch.assert();
    }

    #[tokio::test]
    async fn test_qualys_get_multiple_batches_with_rate_limit() {
        let (mut qualys, mut mock_server, _tracing_guard) = setup().await;

        let mut cves: Vec<String> = (0..450).map(|x| x.to_string()).collect();
        info!(cves_len=?cves.len());
        let query = format!("action=list&details=All&cve={}", cves.join(","));

        let mock_first_batch = mock_server
            .mock("GET", "/api/2.0/fo/knowledge_base/qvs/")
            .match_query(query.as_str())
            .with_status(200)
            .with_body(mock_response_body())
            .with_header("X-RateLimit-Remaining", "0")
            .with_header("X-RateLimit-ToWait-Sec", "1")
            .expect(1)
            .create();

        let mock_second_batch = mock_server
            .mock("GET", "/api/2.0/fo/knowledge_base/qvs/")
            .match_query("action=list&details=All&cve=450")
            .with_status(200)
            .with_body(mock_response_body())
            .expect(1)
            .create();

        cves.push("450".to_string());

        let before = Instant::now();
        let _ = qualys.get_qvs(&cves).await.unwrap();
        let after = Instant::now();
        let duration = after - before;

        mock_first_batch.assert();
        mock_second_batch.assert();

        assert!(duration > tokio::time::Duration::from_secs(1));
    }
}
