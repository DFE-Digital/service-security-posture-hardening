use std::sync::Arc;

use anyhow::Result;
use data_ingester_splunk::splunk::{set_ssphp_run, Splunk};
use data_ingester_supporting::keyvault::Secrets;
use serde::Serialize;

static SSPHP_RUN_KEY: &str = "censys";

async fn entrypoint(secrets: Arc<Secrets>, splunk: Arc<Splunk>) -> Result<()> {
    set_ssphp_run(crate::SSPHP_RUN_KEY)?;
    
    Ok(())
}

struct CensysApi {
    api_id: String,
    secret: String,
    
}

impl V2::CensysApi for CensysApi {
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

    pub(crate) trait CensysApi {
        fn api_id(&self) -> &str;
        fn secret(&self) -> &str;
        async fn post_hosts_search(&self, request_body: &HostSearchRequest) -> Result<()>{
            let client = reqwest::Client::new();
            let url = "";
            let response = client.post(url).json(&request_body).basic_auth(self.api_id(), Some(self.secret())).send().await?;
            Ok(())
        }
    }

    #[derive(Debug, Serialize, Default)]
    struct HostSearchRequest {
        // Query used to search for certificates with matching attributes. Uses the Censys Search Language.
        q: String,
        // The maximum number of hits to return in each response (minimum of 1, maximum of 100).
        per_page: u8,
        // Determine how to query Virtual Hosts. The default is EXCLUDE
        // which will ignore any virtual hosts entries. When set to
        // INCLUDE or ONLY virtual hosts will be present in the returned
        // list of hits, with the later returning only virtual hosts.
        virtual_hosts: HostSearchRequestVirtualHosts,
        // Cursor token from the API response, which fetches the next or previous page of hits when added to the endpoint URL.
        cursor: String,
        // Sort the results
        sort: HostSearchRequestSort,
        // Comma separated list of up to 25 fields to be returned for each result. (This parameter is only available to paid users.)
        fields: String,
    }

    #[derive(Debug, Serialize, Default)]
    #[serde(rename_all="UPPERCASE")]
    enum HostSearchRequestVirtualHosts {
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
    struct HostSearchResponse {
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

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
