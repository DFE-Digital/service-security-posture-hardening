use std::sync::Arc;

use anyhow::{Context, Result};
use data_ingester_splunk::splunk::{try_collect_send, Splunk, ToHecEvents};
use data_ingester_supporting::keyvault::Secrets;
use reqwest::{
    header::{HeaderMap, HeaderValue},
    Client, Method, RequestBuilder,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::info;

pub async fn entrypoint(secrets: Arc<Secrets>, splunk: Arc<Splunk>) -> Result<()> {
    let sonar_api_key = secrets
        .sonar_api_key
        .as_ref()
        .expect("Sonar API Key should be configured");

    let sonar = Sonar::new(sonar_api_key)?;

    let orgs = secrets
        .sonar_orgs
        .as_ref()
        .expect("Sonar orgs should be configured")
        .clone();

    for org in orgs {
        let _project_list = try_collect_send(
            &format!("SonarCloud project list for {org}"),
            sonar.list_projects(&org),
            &splunk,
        )
        .await;
    }
    Ok(())
}

#[derive(Debug, Default, Clone)]
struct Sonar {
    client: Client,
}

impl Sonar {
    /// Create a new Sonar Cloud client using basic auth
    fn new(bearer: &str) -> Result<Self> {
        let client = reqwest::ClientBuilder::new()
            .default_headers(Sonar::headers(bearer).context("Building SonarCloud headers")?)
            .build()
            .context("Building SonarCloud reqwest client")?;
        info!("Qualys client: {:?}", client);
        Ok(Self { client })
    }

    /// RequestBuilder utilising basic_auth
    fn request(&self, method: Method, url: &str) -> RequestBuilder {
        self.client.request(method, url)
    }

    /// Default headers
    fn headers(bearer: &str) -> Result<HeaderMap> {
        let mut headers = HeaderMap::new();

        let user_agent = HeaderValue::from_str("curl/8.4.0")?;
        _ = headers.insert("User-Agent", user_agent);

        let mut authorization = HeaderValue::from_str(bearer)?;
        authorization.set_sensitive(true);
        _ = headers.insert(reqwest::header::AUTHORIZATION, authorization);

        let content_type = HeaderValue::from_str("application/json")?;
        _ = headers.insert("Content-Type", content_type);
        Ok(headers)
    }

    async fn list_projects(&self, org: &str) -> Result<SonarResponse> {
        let url = "https://sonarcloud.io/api/components/search_projects";
        let org = format!("?organization={org}");
        let mut page_n = 1;

        let mut sonar_response = SonarResponse {
            paging: SonarPaging {
                page_index: 0,
                page_size: 0,
                total: 0,
            },
            components: vec![],
            source: format!("{url}{org}"),
        };

        loop {
            let page = format!("&ps=300&p={page_n}");
            let request_url = format!("{url}{org}{page}");
            let response = self.request(Method::GET, &request_url).send().await?;
            let body = response.text().await?;
            let page_response = serde_json::from_str::<SonarResponse>(&body)?;
            sonar_response.components.extend(page_response.components);
            sonar_response.paging = page_response.paging;
            if sonar_response.paging.total == sonar_response.components.len() {
                break;
            }
            page_n += 1;
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        }
        Ok(sonar_response)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct SonarResponse {
    paging: SonarPaging,
    components: Vec<Value>,
    #[serde(default)]
    source: String,
}

impl ToHecEvents for &SonarResponse {
    type Item = Value;

    fn source(&self) -> &str {
        self.source.as_str()
    }

    fn sourcetype(&self) -> &str {
        "sonar_cloud"
    }

    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        Box::new(self.components.iter())
    }

    fn ssphp_run_key(&self) -> &str {
        "sonar_cloud"
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
struct SonarPaging {
    page_index: i32,
    page_size: i32,
    total: usize,
}

#[cfg(feature = "live_tests")]
#[cfg(test)]
mod test {
    use std::env;

    use anyhow::{Context, Result};
    use data_ingester_splunk::splunk::{Splunk, ToHecEvents};
    use data_ingester_supporting::keyvault::get_keyvault_secrets;

    use crate::Sonar;

    #[derive(Clone)]
    struct TestClient {
        client: Sonar,
        orgs: Vec<String>,
        splunk: Splunk,
    }

    impl TestClient {
        async fn new() -> Result<TestClient> {
            let secrets = get_keyvault_secrets(
                &env::var("KEY_VAULT_NAME").expect("Need KEY_VAULT_NAME enviornment variable"),
            )
            .await?;

            let splunk = Splunk::new(
                secrets.splunk_host.as_ref().context("No value")?,
                secrets.splunk_token.as_ref().context("No value")?,
            )?;

            let sonar_api_key = secrets
                .sonar_api_key
                .as_ref()
                .expect("Sonar API Key should be configured");
            let sonar = Sonar::new(sonar_api_key)?;

            let orgs = secrets
                .sonar_orgs
                .as_ref()
                .expect("Sonar orgs should be configured")
                .clone();

            Ok(TestClient {
                client: sonar,
                orgs,
                splunk,
            })
        }
    }

    #[tokio::test]
    async fn test_list_projects() -> Result<()> {
        let test_client = TestClient::new().await?;
        for org in test_client.orgs {
            let result = test_client.client.list_projects(&org).await?;
            assert!(result.paging.total > 0);
            let hec_events = (&result).to_hec_events()?;
            test_client.splunk.send_batch(&hec_events).await?;
        }
        Ok(())
    }
}
