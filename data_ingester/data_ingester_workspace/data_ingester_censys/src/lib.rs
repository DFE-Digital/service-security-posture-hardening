use std::sync::Arc;

use V2::CensysApiTrait;
use anyhow::{Context, Result};
use data_ingester_splunk::splunk::{HecEvent, Splunk, SplunkTrait, get_ssphp_run, set_ssphp_run};
use data_ingester_supporting::keyvault::Secrets;
use hickory_proto::rr::record_type;
use hickory_resolver::lookup;
use serde::Serialize;
use tracing::warn;
use tracing::{error, info};
use valuable::Value;

use hickory_proto::rr::RData;
use hickory_proto::rr::record_type::RecordType;
use hickory_resolver::Name;
use hickory_resolver::TokioResolver;
use hickory_resolver::config::*;

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
        fields: Some(
            //[
            vec![
                "autonomous_system.name".into(),
                "dns.names".into(),
                "ip".into(),
                "location.province".into(),
                "services.extended_service_name".into(),
                "services.labels".into(),
                "services.port".into(),
                "services.software.uniform_resource_identifier".into(),
                "services.tls.certificate.names".into(),
                "services.tls.certificate.parsed.validity_period.not_after".into(),
                "whois.network.cidrs".into(),
            ],
        ),
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
                "censys:json",
                "ssphp_test",
                ssphp_run,
            )
            .ok()
        })
        .collect::<Vec<HecEvent>>();

    dbg!(&hec_events);

    //splunk.send_batch(hec_events).await;

    let hosts: Vec<V2::HostResult> = results
        .into_iter()
        .flat_map(|value| {
            let host = serde_json::from_value::<V2::HostResult>(value);
            match host {
                Ok(host) => Some(host),
                Err(err) => {
                    error!(error=?err, "Error converting Value into V2::HostResult");
                    return None;
                }
            }
        })
        .collect();

    //let mut hec_events: Vec<HecEvent> = vec![];

    let resolver = TokioResolver::tokio(ResolverConfig::default(), ResolverOpts::default());

    let record_types = [
        RecordType::A,
        RecordType::MX,
        RecordType::NS,
        RecordType::AAAA,
        RecordType::CNAME,
        RecordType::TXT,
    ];

    for host in hosts {
        for name in host.tls_names() {
            for port in host.ports_http_https() {
                info!(virtual_host = name, host_ip = host.ip);
                let http_response = match host.http_request(port, &name).await {
                    Ok(response) => response,
                    Err(err) => {
                        error!(error=?err, virtual_host=name, "Error sending HTTP request");
                        continue;
                    }
                };
                let source = format!("{}:{}:{}", &host.ip, port.to_string(), &name);
                let hec_event = HecEvent::new_with_ssphp_run_index(
                    &http_response,
                    source,
                    "censys:json:http_response",
                    "ssphp_test",
                    ssphp_run,
                )?;
                //dbg!(&hec_event);
                splunk.send_batch([hec_event]).await;
            }
        }

        for name in host.all_names() {
            for record_type in record_types {
                let lookup_result = match resolver.lookup(name, record_type).await {
                    Ok(result) => result,
                    Err(err) => {
                        warn!(name=name, record_type=?record_type, "Unable to lookup record");
                        continue;
                    }
                };
                dbg!(&lookup_result);
                let hec_events: Vec<HecEvent> = lookup_result
                    .records()
                    .iter()
                    .map(|record| {
                        let source = format!("{}:{}:{}", &host.ip, &name, &record_type);
                        let hec_event = HecEvent::new_with_ssphp_run_index(
                            &record,
                            source,
                            "censys:json:dns_lookup",
                            "ssphp_test",
                            ssphp_run,
                        )
                        .unwrap();
                        dbg!(&hec_event);
                        hec_event
                    })
                    .collect();
                splunk.send_batch(hec_events).await;
            }
        }
    }

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
    use std::{collections::HashMap, net::SocketAddr};
    use tracing::{error, info};
    use valuable::Valuable;

    #[derive(Debug, Deserialize)]
    pub struct HostResult {
        pub ip: String,
        dns: Option<HostResultDns>,
        pub(crate) services: Vec<HostResultService>,
    }

    impl HostResult {
        pub fn all_names(&self) -> Vec<&str> {
            let dns_names_iter = self
                .dns
                .as_ref()
                .map(|dns| dns.names.iter())
                .or(Some([].iter()))
                .expect(".or() should always work");

            let tls_names_iter = self
                .services
                .iter()
                .filter_map(|service| service.tls.as_ref())
                .flat_map(|tls| tls.certificate.names.iter());
            dns_names_iter
                .chain(tls_names_iter)
                .map(|name| name.as_str())
                .collect()
        }

        pub fn ports(&self) -> Vec<u16> {
            self.services.iter().map(|service| service.port).collect()
        }

        pub fn ports_http_https(&self) -> Vec<u16> {
            self.services
                .iter()
                .filter(|service| service.extended_service_name.contains("HTTP"))
                .map(|service| service.port)
                .collect()
        }

        pub fn tls_names(&self) -> Vec<String> {
            self.services
                .iter()
                .flat_map(|service| {
                    service
                        .tls
                        .as_ref()
                        .map(|tls| tls.certificate.names.clone())
                })
                .flatten()
                .collect()
        }

        pub async fn http_request(&self, port: u16, virtual_host: &str) -> Result<HttpResponse> {
            let socket: SocketAddr = format!("{}:{}", self.ip, port).parse()?;
            let client = reqwest::ClientBuilder::new()
                .resolve(virtual_host, socket)
                .redirect(reqwest::redirect::Policy::none())
                .build()?;

            let url = format!("http://{virtual_host}:{port}/");
            let response = client.get(&url).send().await?;
            let headers = response
                .headers()
                .iter()
                .map(|(header_name, header_value)| {
                    (
                        header_name.as_str().to_owned(),
                        // This should be most robust
                        header_value.to_str().unwrap_or_default().to_owned(),
                    )
                })
                .collect::<HashMap<String, String>>();
            let status = response.status();
            let body = response.text().await?;
            let http_response = HttpResponse {
                request: HttpRequest {
                    host: virtual_host.into(),
                    ip: self.ip.to_owned(),
                    port: port,
                    url: url.into(),
                },
                headers: headers,
                status: status.into(),
                body,
            };
            Ok(http_response)
        }
    }

    #[derive(Debug, Deserialize, Serialize)]
    pub struct HttpResponse {
        request: HttpRequest,
        headers: HashMap<String, String>,
        status: u16,
        body: String,
    }

    #[derive(Debug, Deserialize, Serialize)]
    struct HttpRequest {
        host: String,
        ip: String,
        port: u16,
        url: String,
    }

    #[derive(Debug, Deserialize, Serialize)]
    struct HostResultDns {
        names: Vec<String>,
    }

    #[derive(Debug, Deserialize)]
    struct HostResultService {
        pub(crate) extended_service_name: String,
        labels: Vec<String>,
        port: u16,
        tls: Option<HostResultSerivceTls>,
    }

    #[derive(Debug, Deserialize)]
    struct HostResultSerivceTls {
        certificate: HostResultServiceTlsCertificate,
    }

    #[derive(Debug, Deserialize)]
    struct HostResultServiceTlsCertificate {
        names: Vec<String>,
    }

    pub(crate) trait CensysApiTrait {
        fn api_id(&self) -> &str;
        fn secret(&self) -> &str;
        async fn post_hosts_search(
            &self,
            request_body: &mut HostSearchRequest,
        ) -> Result<Vec<Value>> {
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
        pub(crate) fields: Option<Vec<String>>,
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

    // #[derive(Debug, Deserialize, Default)]
    // struct HostSearchResponseHit {
    //     dns: HostSearchResponseHitDns,
    //     services: Vec<HostSearchResponseHitService>,
    //     #[serde(flatten)]
    //     extra: HashMap<Value, Value>,
    // }

    // impl HostSearchResponseHit {
    //     fn dns_names(&self) -> Vec<&str> {
    //         let dns_names_iter = self.dns.names
    //             .iter();
    //         let tls_names_iter = self.services
    //             .iter()
    //             .filter_map(|service| service.tls.as_ref())
    //             .flat_map(|tls| tls.certificate.names.iter());
    //         dns_names_iter
    //             .chain(tls_names_iter)
    //             .map(|name| name.as_str())
    //             .collect()
    //     }
    // }

    // #[derive(Debug, Deserialize, Default)]
    // struct HostSearchResponseHitDns {
    //     names: Vec<String>,
    // }

    // #[derive(Debug, Deserialize, Default)]
    // struct HostSearchResponseHitService {
    //     extende_service_name: String,
    //     port: u16,
    //     tls: Option<HostSearchResponseHitServiceTls>,
    //     #[serde(flatten)]
    //     extra: HashMap<Value, Value>,
    // }

    // #[derive(Debug, Deserialize, Default)]
    // struct HostSearchResponseHitServiceTls {
    //     certificate: HostSearchResponseHitServiceTlsCertificate,
    //     #[serde(flatten)]
    //     extra: HashMap<Value, Value>,
    // }

    // #[derive(Debug, Deserialize, Default)]
    // struct HostSearchResponseHitServiceTlsCertificate {
    //     names: Vec<String>,
    //     #[serde(flatten)]
    //     extra: HashMap<Value, Value>,
    // }

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

    // #[tokio::test]
    // async fn run_entrypoint() {
    //     entrypoint().await;
    //     assert!(false);
    // }

    #[tokio::test]
    async fn test_resolver() {
        let resolver = TokioResolver::tokio(ResolverConfig::default(), ResolverOpts::default());
        let name = "alexa.kinnane.io";
        let record_types = [RecordType::A];
        for record_type in record_types {
            let lookup_result = match resolver.lookup(name, record_type).await {
                Ok(result) => result,
                Err(err) => {
                    warn!(name=name, record_type=?record_type, "Unable to lookup record");
                    continue;
                }
            };
            dbg!(lookup_result);
        }
        assert!(false);
    }
}

// mod dns {
//     use hickory_proto::rr::record_type::RecordType;
//     use hickory_proto::rr::RData;
//     use hickory_resolver::config::*;
//     use hickory_resolver::Name;
//     use hickory_resolver::TokioResolver;

//     struct Dns {
//         name: String,
//         resolver: &TokioResolver
//     }

//     impl Dns {
//         fn new(resolver: () name: &str) -> Self {
//             Self {
//                 resolver,
//                 name: name.into()
//             }
//         }

//         async fn lookup_dns_records()
//     }

// }
