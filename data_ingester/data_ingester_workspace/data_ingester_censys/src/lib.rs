use std::sync::Arc;

use anyhow::Result;
use data_ingester_splunk::splunk::{set_ssphp_run, Splunk};
use data_ingester_supporting::keyvault::Secrets;
use serde::Serialize;
use tracing::info;
use valuable::Value;
use V2::CensysApiTrait;

static SSPHP_RUN_KEY: &str = "censys";

// async fn entrypoint(secrets: Arc<Secrets>, splunk: Arc<Splunk>) -> Result<()> {
    async fn entrypoint() -> Result<()> {    
    info!(name=SSPHP_RUN_KEY, "Starting");
    set_ssphp_run(crate::SSPHP_RUN_KEY)?;

    //let api_id = secrets.
    let api_id = "api_id";
    let secret = "secret";

    let client = CensysApi {
        api_id: api_id.to_string(),
        secret: secret.to_string(),
    };

        dbg!(1);

    let request = V2::HostSearchRequest {
        q: ".sch.uk".to_string(),
        per_page: 1,
        virtual_hosts: V2::HostSearchRequestVirtualHosts::Include,
        .. Default::default()
    };
                dbg!(2);

        let results = client.post_hosts_search(&request).await?;
        
            dbg!(3);
    Ok(())
}

struct CensysApi {
    api_id: String,
    secret: String,
}

impl CensysApiTrait for CensysApi {
    fn api_id(&self) -> &str{
        &self.api_id
    }
    fn secret(&self) -> &str {
        &self.secret
    }
}

mod V2 {
    use serde::{Deserialize, Serialize};
    use serde_json::Value;
    use anyhow::Result;
    use tracing::error;
    use valuable::Valuable;

    pub(crate) trait CensysApiTrait {
        fn api_id(&self) -> &str;
        fn secret(&self) -> &str;
        async fn post_hosts_search(&self, request_body: &HostSearchRequest) -> Result<HostSearchResponse>{
            let client = reqwest::Client::new();
            let url = "https://search.censys.io/v2/hosts/search";
            dbg!(&url);
            let response = client.post(url)
                .json(&request_body)
                //.basic_auth(self.api_id(), Some(self.secret()))
                .send().await?;
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
            let mut host_search_response: HostSearchResponse = serde_json::from_str(&response_body_text)?;
            dbg!(&host_search_response);

            
            Ok(host_search_response)
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
        pub(crate) fields: String,
    }

    #[derive(Debug, Serialize, Default)]
    #[serde(rename_all="UPPERCASE")]
    pub(crate) enum HostSearchRequestVirtualHosts {
        #[default]
        Exclude,
        Include,
        Only,
    }

    #[derive(Debug, Serialize, Default)]
    #[serde(rename_all="UPPERCASE")]
    enum HostSearchRequestSort {
        #[default]
        Relevance,
        Ascending,
        Descending    
    }

    #[derive(Debug, Deserialize, Default)]    
    pub(crate) struct HostSearchResponse {
        // HTTP Status code
        code: u16,
        // Status
        status: String,
        // Result
        result: HostSearchResponseResult,
        // links
        links: HostSearchResponseLinks,
    }

    #[derive(Debug, Deserialize, Default)]    
    struct HostSearchResponseResult {
        query: String,
        total: usize,
        // Vec of HostHit or VirtualHostHit
        hits: Vec<Value>
    }

    #[derive(Debug, Deserialize, Default)]    
    struct HostSearchResponseLinks {
        prev: String,
        next: String,
    }

    
}


#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn run_entrypoint() {
        entrypoint().await;
        assert!(false);
    }
}
