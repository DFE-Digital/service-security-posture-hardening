use std::sync::Arc;

use V2::CensysApiTrait;
use anyhow::{Context, Result};
use data_ingester_splunk::splunk::{HecEvent, Splunk, SplunkTrait, get_ssphp_run, set_ssphp_run};
use data_ingester_supporting::keyvault::Secrets;
use serde::Serialize;
use tracing::info;
use valuable::Value;

static SSPHP_RUN_KEY: &str = "censys";

pub async fn entrypoint(secrets: Arc<Secrets>, splunk: Arc<Splunk>) -> Result<()> {
    info!(name = SSPHP_RUN_KEY, "Starting");

    let _ = set_ssphp_run(crate::SSPHP_RUN_KEY)?;

    let api_id = secrets
        .censys_api
        .as_ref()
        .context("Expect Censys API ID secret")?;

    let secret = secrets
        .censys_secret
        .as_ref()
        .context("Expect Censys secret")?;

    let client = CensysApi {
        api_id: api_id.to_string(),
	// api_id: "FOOFOO".to_string(),
       secret: secret.to_string(),
       // secret: "BARBAR".to_string(),	
    };

    dbg!(1);

    let mut request = V2::HostSearchRequest {
        q: "sch.uk".to_string(),
        per_page: 100,
        virtual_hosts: V2::HostSearchRequestVirtualHosts::Include,
	fields: Some(["dns.names",
		 "ip",
		 "services.port","services.extended_service_name",
		 "services.tls.certificate.names",
		 "autonomous_system.name",
		 "location.province",
		 "services.labels",
		 "services.software.uniform_resource_identifier",
		 "services.tls.certificate.parsed.validity_period.not_after",
		 "whois.network.cidrs"].join(",")),
        ..Default::default()
    };
    dbg!(2);

    let results = client.post_hosts_search(&mut request).await?;
    dbg!(&results);
    dbg!(3);

    let ssphp_run = get_ssphp_run(SSPHP_RUN_KEY);
    let q = request.q.to_owned();
    let hec_events = results
        .iter()
        .filter_map(|result| {
            HecEvent::new_with_ssphp_run_index(
                result,
                q.clone(),
                "censys",
                "ssphp_test",
                ssphp_run,
            )
            .ok()
        })
        .collect::<Vec<HecEvent>>();

    splunk.send_batch(hec_events).await;

    Ok(())
}

struct CensysApi {
    api_id: String,
    secret: String,
}

impl CensysApiTrait for CensysApi {
    fn api_id(&self) -> &str {
        &self.api_id
    }
    fn secret(&self) -> &str {
        &self.secret
    }
}

mod V2 {
    use anyhow::Result;
    use serde::{Deserialize, Serialize};
    use serde_json::Value;
    use tracing::{error, info};
    use valuable::Valuable;

    pub(crate) trait CensysApiTrait {
        fn api_id(&self) -> &str;
        fn secret(&self) -> &str;
        async fn post_hosts_search(&self, request_body: &mut HostSearchRequest) -> Result<Vec<Value>> {
            let client = reqwest::Client::new();
            let url = "https://search.censys.io/api/v2/hosts/search";
            dbg!(&url);
            let response = client
                .post(url)
                .json(&request_body)
                .basic_auth(self.api_id(), Some(self.secret()))
                .send()
                .await?;
            dbg!("p1");
            let response_status = response.status().as_u16();
            dbg!(&response_status);
            let response_body_text = response.text().await?;
            dbg!(&response_body_text);
            if response_status > 300 {
                let error_body: Value = serde_json::from_str(&response_body_text)?;
                dbg!(&error_body);
                error!(request=?request_body, response_status=?response_status, response_body=?error_body, "Failed to make request CenSys request");
                anyhow::bail!("censys request failed");
            }
            let mut host_search_response: HostSearchResponse =
                serde_json::from_str(&response_body_text)?;
            dbg!(&host_search_response);

            let mut results = vec![];
            results.append(&mut host_search_response.result.hits);
            let mut iter_count = 0;
            loop {
                if host_search_response.result.links.next.is_empty() {
                    info!("next is empty");
                    break;
                }
                if iter_count >= 3 {
                    info!("iter_count exceeded");
                    break;
                }

                request_body.cursor = Some(host_search_response.result.links.next.to_owned());

                let response = client
                    .post(url)
                    .json(&request_body)
                    .basic_auth(self.api_id(), Some(self.secret()))
                    .send()
                    .await?;
                dbg!("p1");

                let response_status = response.status().as_u16();
                dbg!(&response_status);

                let response_body_text = response.text().await?;
                dbg!(&response_body_text);

                if response_status > 300 {
                    let error_body: Value = serde_json::from_str(&response_body_text)?;
                    dbg!(&error_body);
                    error!(request=?request_body, response_status=?response_status, response_body=?error_body, "Failed to make request CenSys request");
                    anyhow::bail!("censys request failed");
                }
                host_search_response = serde_json::from_str(&response_body_text)?;
                dbg!(&host_search_response);

                results.append(&mut host_search_response.result.hits);
                iter_count += 1;
            }

            Ok(results)
        }
    }

    #[derive(Debug, Serialize, Default)]
    pub(crate) struct HostSearchRequest {
        // Query used to search for certificates with matching attributes. Uses the Censys Search Language.
        pub(crate) q: String,
        // The maximum number of hits to return in each response (minimum of 1, maximum of 100).
        pub(crate) per_page: u8,
        // Determine how to query Virtual Hosts. The default is EXCLUDE
        // which will ignore any virtual hosts entries. When set to
        // INCLUDE or ONLY virtual hosts will be present in the returned
        // list of hits, with the later returning only virtual hosts.
        pub(crate) virtual_hosts: HostSearchRequestVirtualHosts,
        // Cursor token from the API response, which fetches the next or previous page of hits when added to the endpoint URL.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub(crate) cursor: Option<String>,
        // Sort the results
        pub(crate) sort: HostSearchRequestSort,
        // Comma separated list of up to 25 fields to be returned for each result. (This parameter is only available to paid users.)
        #[serde(skip_serializing_if = "Option::is_none")]
        pub(crate) fields: Option<String>,
    }

    #[derive(Debug, Serialize, Default)]
    #[serde(rename_all = "UPPERCASE")]
    pub(crate) enum HostSearchRequestVirtualHosts {
        #[default]
        Exclude,
        Include,
        Only,
    }

    #[derive(Debug, Serialize, Default)]
    #[serde(rename_all = "UPPERCASE")]
    enum HostSearchRequestSort {
        #[default]
        Relevance,
        Ascending,
        Descending,
    }

    #[derive(Debug, Deserialize, Default)]
    pub(crate) struct HostSearchResponse {
        // HTTP Status code
        code: u16,
        // Status
        status: String,
        // Result
        result: HostSearchResponseResult,
    }

    #[derive(Debug, Deserialize, Default)]
    struct HostSearchResponseResult {
        query: String,
        total: usize,
        // Vec of HostHit or VirtualHostHit
        hits: Vec<Value>,
        // links
        links: HostSearchResponseLinks,
    }

    #[derive(Debug, Deserialize, Default)]
    struct HostSearchResponseLinks {
        prev: String,
        next: String,
    }
}

#[cfg(test)]
mod tests {

    //  "{\"code\": 200, \"status\": \"OK\", \"result\": {\"query\": \"foo.sch.uk\", \"total\": 0, \"duration\": 105, \"hits\": [], \"links\": {\"next\": \"\", \"prev\": \"\"}}}"
    use super::*;

    #[tokio::test]
    async fn run_entrypoint() {
        entrypoint().await;
        assert!(false);
    }
}
