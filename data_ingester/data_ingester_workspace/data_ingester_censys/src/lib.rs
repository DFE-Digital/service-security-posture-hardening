use std::collections::HashSet;
use std::sync::Arc;

use V2::{CensysApiTrait, HostResult, convert_txt_record_data_to_ascii};
use anyhow::{Context, Result};
use data_ingester_splunk::splunk::{HecEvent, Splunk, SplunkTrait, get_ssphp_run, set_ssphp_run};
use data_ingester_supporting::keyvault::Secrets;
use futures::FutureExt;
use futures::future::join_all;
use hickory_proto::rr::record_type;
use hickory_resolver::lookup;
use itertools::{Itertools, interleave};
use serde::Serialize;
use tracing::warn;
use tracing::{error, info};
use valuable::Value;

use V2::PendingHttpRequest;
use V2::ToSplunk;
use futures::stream::{self, FuturesUnordered, StreamExt};
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

    let http_requests = hosts
        .iter()
        .flat_map(|host| host.request_all_http_ports_all_virtual_hosts())
        .map(|request| request.run_request());

    let mut stream = stream::iter(http_requests)
        .buffer_unordered(10)
        .filter_map(|response| async { response.complete_processing() })
        .filter_map(|response| async move {
            let hec_event_result = HecEvent::new_with_ssphp_run_index(
                &response.response,
                response.source(),
                response.sourcetype(),
                "ssphp_test",
                ssphp_run,
            );
            match hec_event_result {
                Ok(hec_event) => Some(hec_event),
                Err(err) => {
                    warn!(error=?err, response=?response, "Unable to create HecEvent");
                    None
                }
            }
        })
        .map(|hec_event| splunk.send_batch([hec_event]))
        .buffer_unordered(10);

    while let Some(fut) = stream.next().await {
        dbg!(fut);
    }

    Ok(())
}

// async fn collect_hosts_http(hosts: Vec<V2::HostResult>, splunk: Arc<Splunk>, ssphp_run: u64) -> () {
//     let http_request = async |ip: String, name: String, port: u16| {
//         info!(virtual_host = name, host_ip = ip);
//         let http_response = match host.http_request(port, &name).await {
//             Ok(response) => response,
//             Err(err) => {
//                 error!(error=?err, virtual_host=name, "Error sending HTTP request");
//                 return ();
//             }
//         };
//         let source = format!("{}^{}^{}", &ip, port.to_string(), &name);
//         let hec_event = HecEvent::new_with_ssphp_run_index(
//             &http_response,
//             source,
//             "censys:json:http_response",
//             "ssphp_test",
//             ssphp_run,
//         )
//             .unwrap();
//         //dbg!(&hec_event);
//         let _ = splunk.send_batch([hec_event]).await;
//     };

//     let http_stream = stream::iter(
//         hosts
//             .clone()
//             .into_iter()
//             .flat_map(|host| {
//                 let names: Vec<String> = host.all_names().iter().map(|s| s.to_string()).collect();
//                 //                let ha = host.clone();
//                 names.into_iter().map(move |name| (host.clone(), name))
//             })
//             .flat_map(|(host, name)| {
//                 host.ports_http_https()
//                     .into_iter()
//                     .map(move |port| (host.ip.to_string(), name.clone(), port))
//             })
//             //.inspect(|(ip, name, port)| {dbg!(&ip, &name, &port);})
//             .map(|(ip, name, port)| async move { dbg!(ip, name, port) }),
//             .map(|(ip, name, port)| async move { http_request(host }),
//     )
//     .buffer_unordered(2)
//     .for_each(|n| async {})
//             .boxed()
//     }
// }

// async fn collect_hosts_http(hosts: Vec<V2::HostResult>, splunk: Arc<Splunk>, ssphp_run: u64) -> () {

//     let record_types = [
//         RecordType::A,
//         RecordType::MX,
//         RecordType::NS,
//         RecordType::AAAA,
//         RecordType::CNAME,
//         RecordType::TXT,
//     ];

//     let mut names_seen = HashSet::<String>::new();

//     let resolver = TokioResolver::tokio_from_system_conf().unwrap();

//     let dns_stream = stream::iter(
//         hosts
//             .iter()
//             .flat_map(|host| {
//                 host.all_names()
//                     .into_iter()
//                     .map(|name| (host.ip.to_string(), name.to_string()))
//             })
//             .filter(move |(ip, name): &(String, String)| names_seen.insert(name.to_string()))
//             .cartesian_product(record_types)
//             .map(|((ip, name), record_type)| async move { dbg!(ip, name, record_type) }),
//         // .inspect(|((ip, name), record_type)| resolve_records(ip.to_string(), name.to_string(), record_type.clone()))
//         // .map(|((ip, name), record_type)| resolve_records(ip.to_string(), name.to_string(), record_type.clone()))
//         //            .map(|a| ())
//     )
//     .buffer_unordered(2)
//     .for_each(|n| async {})
//     .boxed();

//     join_all([http_stream, dns_stream]).await;
// }

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
    use data_ingester_splunk::splunk::HecEvent;
    use futures::{FutureExt, future::BoxFuture};
    use itertools::Itertools;
    use serde::{Deserialize, Serialize};
    use serde_json::Value;
    use tokio::time::timeout;
    use std::{collections::HashMap, net::SocketAddr, time::Duration};
    use tracing::{error, info, warn};
    use valuable::Valuable;

    use crate::SSPHP_RUN_KEY;

    #[derive(Debug, Deserialize, Clone)]
    pub struct HostResult {
        pub ip: Ip,
        pub(crate) dns: Option<HostResultDns>,
        pub(crate) services: Vec<HostResultService>,
    }
    #[derive(Debug, Serialize, Deserialize, Clone)]
    struct Ip(String);
    #[derive(Debug, Serialize, Deserialize, Clone)]
    struct VHost(String);
    #[derive(Debug, Serialize, Deserialize, Clone)]
    struct Port(u16);

    //#[derive()]
    pub struct PendingHttpRequest<'a> {
        ip: &'a Ip,
        vhost: &'a VHost,
        port: &'a Port,
        request: BoxFuture<'a, Result<HttpResponse>>,
    }

    impl<'a> PendingHttpRequest<'a> {
        pub async fn run_request(self) -> ProcessingHttpRequest<'a> {
            ProcessingHttpRequest {
                ip: self.ip,
                vhost: self.vhost,
                port: self.port,
                response: self.request.await,
            }
        }
    }

    #[derive(Debug)]
    pub struct ProcessingHttpRequest<'a> {
        ip: &'a Ip,
        vhost: &'a VHost,
        port: &'a Port,
        pub response: Result<HttpResponse>,
    }

    impl<'a> ProcessingHttpRequest<'a> {
        pub fn complete_processing(self) -> Option<ProcessedHttpRequest> {
            if let Ok(response) = self.response {
                Some(ProcessedHttpRequest {
                    ip: self.ip.0.to_string(),
                    vhost: self.vhost.0.to_string(),
                    port: self.port.0.to_string(),
                    response: response,
                })
            } else {
                None
            }
        }
    }

    #[derive(Debug)]
    pub struct ProcessedHttpRequest {
        ip: String,
        vhost: String,
        port: String,
        pub response: HttpResponse,
    }

    impl ToSplunk for ProcessedHttpRequest {
        fn source(&self) -> String {
            format!("{}^{}^{}", &self.ip, self.port, self.vhost)
        }

        fn sourcetype(&self) -> String {
            "censys:json:http_response".into()
        }

        //        fn
    }

    pub(crate) trait ToSplunk {
        fn source(&self) -> String;

        fn sourcetype(&self) -> String;
    }

    impl HostResult {
        pub fn all_names(&self) -> impl Iterator<Item = &VHost> {
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
            dns_names_iter.chain(tls_names_iter)
        }

        pub fn ports(&self) -> impl Iterator<Item = &Port> {
            self.services.iter().map(|service| &service.port)
        }

        pub fn ports_http_https(&self) -> impl Iterator<Item = &Port> {
            self.services
                .iter()
                .filter(|service| service.extended_service_name.contains("HTTP"))
                .map(|service| &service.port)
        }

        pub fn tls_names(&self) -> impl Iterator<Item = &VHost> {
            self.services
                .iter()
                .flat_map(|service| service.tls.as_ref().map(|tls| tls.certificate.names.iter()))
                .flatten()
        }

        pub fn all_http_ports_all_virtual_hosts_combo(
            &self,
        ) -> impl Iterator<Item = (&VHost, &Port)> {
            self.all_names()
                .cartesian_product(self.ports_http_https().collect::<Vec<_>>())
        }

        pub fn request_all_http_ports_all_virtual_hosts(
            &self,
        ) -> impl Iterator<Item = PendingHttpRequest> {
            self.all_http_ports_all_virtual_hosts_combo()
                .map(|(vhost, port)| PendingHttpRequest {
                    ip: &self.ip,
                    vhost,
                    port: port,
                    request: self.http_request(port, vhost).boxed(),
                })
        }

        async fn process_pending_http_requests<'a>(
            &self,
            pending_requests: impl Iterator<Item = PendingHttpRequest<'a>>,
        ) -> impl Iterator<Item = impl Future<Output = ProcessingHttpRequest<'a>>> {
            pending_requests.map(|request| request.run_request())
        }

        async fn http_request(&self, port: &Port, virtual_host: &VHost) -> Result<HttpResponse> {
            let socket: SocketAddr = format!("{}:{}", self.ip.0, port.0).parse()?;
            let client = reqwest::ClientBuilder::new()
                .resolve(virtual_host.0.as_str(), socket)
                .redirect(reqwest::redirect::Policy::none())
                .connect_timeout(Duration::from_secs(3))
                .timeout(Duration::from_secs(5))
                .build()?;

            let url = format!("http://{}:{}/", virtual_host.0, port.0);
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
                    host: virtual_host.0.to_owned(),
                    ip: self.ip.0.to_owned(),
                    port: port.0,
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

    #[derive(Debug, Deserialize, Serialize, Clone)]
    struct HostResultDns {
        names: Vec<VHost>,
    }

    #[derive(Debug, Deserialize, Clone)]
    struct HostResultService {
        pub(crate) extended_service_name: String,
        labels: Vec<String>,
        port: Port,
        tls: Option<HostResultSerivceTls>,
    }

    #[derive(Debug, Deserialize, Clone)]
    struct HostResultSerivceTls {
        certificate: HostResultServiceTlsCertificate,
    }

    #[derive(Debug, Deserialize, Clone)]
    struct HostResultServiceTlsCertificate {
        names: Vec<VHost>,
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

    /// Rewrite a Hickory DNS Response containing txt data to contain an ASCII string instead
    pub(crate) fn convert_txt_record_data_to_ascii(value: Value) -> Value {
        let mut value = value;
        value
            .as_object_mut()
            .and_then(|obj| obj.get_mut("rdata"))
            .and_then(|rdata| rdata.as_object_mut())
            .and_then(|rdata| rdata.get_mut("TXT"))
            .and_then(|txt| txt.as_object_mut())
            .and_then(|txt| txt.get_mut("txt_data"))
            .and_then(|txt_data| txt_data.as_array_mut())
            .and_then(|txt_data| {
                txt_data.iter_mut().for_each(|mut txt_data_array| {
                    dbg!(&txt_data_array);
                    *txt_data_array = txt_data_array
                        .as_array()
                        .iter()
                        .flat_map(|array| array.iter())
                        .filter_map(|char| char.as_u64().map(|char| char as u8 as char))
                        .collect::<String>()
                        .into();
                });
                None::<()>
            });
        value
    }

    #[cfg(test)]
    mod tests {

        use crate::collect_host;

        //        use super::V2::convert_txt_record_data_to_ascii;
        use super::HostResult;
        use super::HostResultDns;
        use serde_json::json;

        use super::*;

        fn host_result() -> HostResult {
            HostResult {
                ip: "1.2.3.4".into(),
                dns: Some(HostResultDns {
                    names: vec!["google.com".into()],
                }),
                services: vec![HostResultService {
                    extended_service_name: "HTTPS".into(),
                    labels: vec![],
                    port: 443,
                    tls: Some(HostResultSerivceTls {
                        certificate: HostResultServiceTlsCertificate {
                            names: vec![
                                "google.com".into(),
                                "wwww.google.com".into(),
                                "wwww.googe.co.uk".into(),
                            ],
                        },
                    }),
                }],
            }
        }

        #[tokio::test]
        async fn test_collect_hosts() {
            let hosts = vec![host_result()];
            collect_host(hosts).await;
            assert!(false);
        }
    }
}

#[cfg(test)]
mod tests {

    use super::V2::convert_txt_record_data_to_ascii;
    use serde_json::json;

    use super::*;

    #[test]
    fn test_convert_txt_record_data_to_ascii() {
        let value = json!({"rdata": { "TXT": { "txt_data": vec![[ 72, 101, 108, 108, 111, 44, 32, 119, 111, 114, 108, 100, 33]] } } });
        //let value = serde_json::from_str(&json).unwrap();
        let updated_value = convert_txt_record_data_to_ascii(value);
        let new_json = serde_json::to_string(&updated_value).unwrap();
        assert_eq!(
            new_json,
            r#"{"rdata":{"TXT":{"txt_data":["Hello, world!"]}}}"#
        );
    }

    #[tokio::test]
    async fn test_txt_record_resolver() {
        // let resolver = TokioResolver::tokio(ResolverConfig::default(), ResolverOpts::default());
        let resolver = TokioResolver::tokio_from_system_conf().unwrap();
        let name = "google.com";
        let record_types = [RecordType::TXT];
        for record_type in record_types {
            let lookup_result = match resolver.lookup(name, record_type).await {
                Ok(result) => result,
                Err(err) => {
                    warn!(name=name, record_type=?record_type, "Unable to lookup record");
                    continue;
                }
            };
            let value = lookup_result
                .records()
                .iter()
                .inspect(|record| {
                    dbg!(&record);
                })
                .map(|record| serde_json::to_value(&record).unwrap())
                .map(|value| convert_txt_record_data_to_ascii(value))
                .for_each(|value| {});
        }

        assert!(false);
    }

    #[tokio::test]
    async fn test_resolver() {
        // let resolver = TokioResolver::tokio(ResolverConfig::default(), ResolverOpts::default());
        let resolver = TokioResolver::tokio_from_system_conf().unwrap();
        let name = "www.google.com";
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
