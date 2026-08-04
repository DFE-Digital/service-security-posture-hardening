use data_ingester_splunk::splunk::{set_ssphp_run, Splunk, SplunkTrait, ToHecEvents};
use mockito::{Matcher, Server};
use serde_json::{json, Value};

use crate::client::PowerPagesClient;
use crate::metadata::{DataType, PowerPagesMetadata};
use crate::models::{EnvironmentResponse, WebsiteDto};
use crate::response::{CollectionOutcome, PowerPagesApiResult};
use crate::summary::RunSummary;
use crate::SSPHP_RUN_KEY;

fn test_env() -> EnvironmentResponse {
    EnvironmentResponse {
        id: "env-1".into(),
        display_name: "Test Env".into(),
        state: None,
        r#type: None,
        tenant_id: None,
        geo: None,
        url: None,
    }
}

fn test_website() -> WebsiteDto {
    WebsiteDto {
        id: "site-1".into(),
        name: "Test Site".into(),
        website_url: "https://test.example.com".into(),
        environment_id: Some("env-1".into()),
        environment_name: Some("Test Env".into()),
        subdomain: Some("test.example.com".into()),
        custom_host_names: vec![],
        site_visibility: None,
        status: None,
    }
}

fn hec_event_payload(event: &data_ingester_splunk::splunk::HecEvent) -> Value {
    serde_json::from_str(&event.event).expect("parse hec event")
}

fn setup_ssphp_run() {
    let _ = set_ssphp_run(SSPHP_RUN_KEY);
}

#[test]
fn array_response_splits_into_one_event_per_element() {
    setup_ssphp_run();
    let metadata = PowerPagesMetadata {
        data_type: DataType::AllowedIp,
        environment_id: Some("env-1".into()),
        environment_name: Some("Test Env".into()),
        website_id: Some("site-1".into()),
        website_name: Some("Test Site".into()),
        website_url: Some("https://test.example.com".into()),
        request_url: "http://test/ips".into(),
        hostname: None,
        cert_type: None,
        is_deep_scan: false,
    };
    let result = PowerPagesApiResult {
        ssphp_http_status: 200,
        ssphp_collection_outcome: CollectionOutcome::Success,
        response_body: None,
        error_message: None,
        data: Some(json!([
            {"ipAddress": "1.2.3.4", "name": "office"},
            {"ipAddress": "5.6.7.8", "name": "vpn"}
        ])),
        metadata,
    };

    let events = (&result).to_hec_events().expect("split array");
    assert_eq!(events.len(), 2);
    for event in &events {
        let payload = hec_event_payload(event);
        assert_eq!(payload["ssphp_collection_outcome"], "success");
        assert!(payload.get("ipAddress").is_some());
    }
}

#[test]
fn deep_scan_404_emits_single_no_scan_event() {
    setup_ssphp_run();
    let metadata = PowerPagesMetadata::for_website(
        &test_env(),
        &test_website(),
        DataType::DeepScanReport,
        "http://test/deep-scan",
    );
    let result = PowerPagesApiResult::from_http(
        404,
        r#"{"error":{"message":"No scan found"}}"#.into(),
        metadata,
        None,
    );

    assert_eq!(result.ssphp_collection_outcome, CollectionOutcome::NoScan);
    let events = (&result).to_hec_events().expect("no_scan event");
    assert_eq!(events.len(), 1);
    let payload = hec_event_payload(&events[0]);
    assert_eq!(payload["ssphp_http_status"], 404);
    assert_eq!(payload["ssphp_collection_outcome"], "no_scan");
}

#[test]
fn preflight_403_emits_preflight_failed_outcome() {
    setup_ssphp_run();
    let metadata = PowerPagesMetadata {
        data_type: DataType::Preflight,
        environment_id: None,
        environment_name: None,
        website_id: None,
        website_name: None,
        website_url: None,
        request_url: "http://test/environments".into(),
        hostname: None,
        cert_type: None,
        is_deep_scan: false,
    };
    let result = PowerPagesApiResult::from_http(
        403,
        r#"{"error":{"message":"Forbidden"}}"#.into(),
        metadata,
        None,
    );

    assert_eq!(
        result.ssphp_collection_outcome,
        CollectionOutcome::PreflightFailed
    );
    let events = (&result).to_hec_events().expect("preflight failed event");
    assert_eq!(events.len(), 1);
    let payload = hec_event_payload(&events[0]);
    assert_eq!(payload["ssphp_collection_outcome"], "preflight_failed");
}

#[test]
fn empty_websites_list_emits_no_power_pages_sites_event() {
    setup_ssphp_run();
    let result = PowerPagesClient::no_power_pages_sites(&test_env(), "http://test/websites");
    assert_eq!(
        result.ssphp_collection_outcome,
        CollectionOutcome::NoPowerPagesSites
    );
    let events = (&result).to_hec_events().expect("no sites event");
    assert_eq!(events.len(), 1);
    let payload = hec_event_payload(&events[0]);
    assert_eq!(payload["ssphp_collection_outcome"], "no_power_pages_sites");
}

#[tokio::test]
async fn hostnames_collected_before_ssl_bindings() {
    let mut server = Server::new_async().await;
    let base = format!("http://{}", server.host_with_port());
    let client = PowerPagesClient::new_for_test(&base, "test-token");
    let env = test_env();
    let website = test_website();

    let call_order = std::sync::Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
    let order_hostnames = call_order.clone();
    let order_ssl = call_order.clone();

    let _hostnames = server
        .mock(
            "GET",
            Matcher::Regex(r"/powerpages/environments/.+/websites/.+/customDomain".into()),
        )
        .with_status(200)
        .with_body(r#"["www.example.com"]"#)
        .with_header("content-type", "application/json")
        .match_request(move |_| {
            order_hostnames
                .lock()
                .expect("lock")
                .push("hostnames".into());
            true
        })
        .create_async()
        .await;

    let _ssl = server
        .mock(
            "GET",
            Matcher::Regex(r"/powerpages/environments/.+/websites/.+/sslBindings".into()),
        )
        .with_status(200)
        .with_body(r#"[{"thumbprint":"abc"}]"#)
        .with_header("content-type", "application/json")
        .match_request(move |_| {
            order_ssl.lock().expect("lock").push("ssl".into());
            true
        })
        .create_async()
        .await;

    let hostnames = client
        .get_hostnames(&env, &website)
        .await
        .expect("hostnames");
    assert_eq!(hostnames.hostnames(), vec!["www.example.com".to_string()]);

    let _bindings = client
        .get_ssl_bindings(&env, &website, "www.example.com")
        .await
        .expect("ssl bindings");

    let order = call_order.lock().expect("lock");
    assert_eq!(*order, vec!["hostnames", "ssl"]);
}

#[tokio::test]
async fn transport_failure_sends_diagnostic_event() {
    let splunk = Splunk::new("http://127.0.0.1:1", "token", false).expect("splunk client");
    let result = data_ingester_splunk::splunk::try_collect_send(
        "Power Pages transport test",
        async { Err::<PowerPagesApiResult, _>(anyhow::anyhow!("connection refused")) },
        &splunk,
    )
    .await;

    assert!(result.is_err());
}

#[test]
fn run_summary_finish_includes_counters() {
    let mut summary = RunSummary::new();
    summary.environments_total = 2;
    summary.environments_with_sites = 1;
    summary.environments_no_power_pages_sites = 1;
    summary.websites_total = 3;
    summary.endpoints_attempted = 10;
    summary.endpoints_http_error = 1;
    summary.endpoints_no_scan = 2;
    summary.hec_events_sent = 25;
    summary.record_preflight(200);

    let finished = summary.finish();
    let value = serde_json::to_value(&finished).expect("serialize summary");
    assert_eq!(value["environments_total"], 2);
    assert_eq!(value["websites_total"], 3);
    assert_eq!(value["preflight_ok"], true);
    assert_eq!(value["hec_events_sent"], 25);
}
