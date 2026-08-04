// Power Pages ingester — requires the Entra app (ad-client-id) to be registered as a
// Power Platform admin management application and assigned RBAC (e.g. Power Platform Reader).
// Pre-flight 401/403 events in Splunk indicate missing prerequisites.
// See: https://learn.microsoft.com/en-us/power-platform/admin/powerplatform-api-create-service-principal

use std::sync::Arc;

use anyhow::{Context, Result};
use data_ingester_splunk::splunk::{set_ssphp_run, try_collect_send, Splunk, ToHecEvents};
use data_ingester_supporting::keyvault::Secrets;
use tracing::{error, info, warn};

use crate::client::PowerPagesClient;
use crate::metadata::{DataType, PowerPagesMetadata};
use crate::models::{EnvironmentResponse, WebsiteDto};
use crate::response::{CollectionOutcome, PowerPagesApiResult};
use crate::summary::RunSummary;
use crate::SSPHP_RUN_KEY;

fn event_count(result: &PowerPagesApiResult) -> usize {
    result
        .to_hec_events()
        .map(|events| events.len())
        .unwrap_or(0)
}

pub async fn entrypoint(secrets: Arc<Secrets>, splunk: Arc<Splunk>) -> Result<()> {
    let _ = set_ssphp_run(SSPHP_RUN_KEY)?;
    info!("Starting Power Pages collection");

    let client_id = secrets
        .azure_client_id
        .as_ref()
        .context("Expect azure_client_id secret")?;
    let client_secret = secrets
        .azure_client_secret
        .as_ref()
        .context("Expect azure_client_secret secret")?;
    let tenant_id = secrets
        .azure_tenant_id
        .as_ref()
        .context("Expect azure_tenant_id secret")?;

    let client = PowerPagesClient::new(client_id, client_secret, tenant_id)
        .await
        .context("Creating Power Pages client")?;

    let mut summary = RunSummary::new();

    let env_result = match try_collect_send(
        "Power Pages pre-flight environments",
        client.list_environments(),
        &splunk,
    )
    .await
    {
        Ok(result) => result,
        Err(err) => {
            error!(error=?err, "Power Pages environment list failed");
            summary.record_preflight(401);
            let _ = send_summary(&splunk, summary).await;
            return Ok(());
        }
    };

    summary.record_preflight(env_result.ssphp_http_status);
    summary.record_endpoint(
        env_result.ssphp_collection_outcome,
        event_count(&env_result),
    );

    if env_result.ssphp_collection_outcome == CollectionOutcome::PreflightFailed {
        warn!(
            status = env_result.ssphp_http_status,
            "Power Pages pre-flight failed — continuing best-effort"
        );
    }

    let environments: Vec<EnvironmentResponse> = env_result.environments();
    summary.environments_total = environments.len();

    for env in environments {
        collect_environment(&client, &splunk, &env, &mut summary).await;
    }

    send_summary(&splunk, summary).await?;
    Ok(())
}

async fn send_summary(splunk: &Splunk, summary: RunSummary) -> Result<()> {
    let finished = summary.finish();
    let result = PowerPagesApiResult {
        ssphp_http_status: 200,
        ssphp_collection_outcome: CollectionOutcome::Success,
        response_body: None,
        error_message: None,
        data: Some(serde_json::to_value(&finished)?),
        metadata: PowerPagesMetadata {
            data_type: DataType::RunSummary,
            environment_id: None,
            environment_name: None,
            website_id: None,
            website_name: None,
            website_url: None,
            request_url: "synthetic:run_summary".into(),
            hostname: None,
            cert_type: None,
            is_deep_scan: false,
        },
    };
    let _ = try_collect_send("Power Pages run summary", async { Ok(result) }, splunk).await;
    Ok(())
}

async fn collect_environment(
    client: &PowerPagesClient,
    splunk: &Splunk,
    env: &EnvironmentResponse,
    summary: &mut RunSummary,
) {
    let websites_result = match try_collect_send(
        &format!("Power Pages websites for env {}", env.id),
        client.list_websites(env),
        splunk,
    )
    .await
    {
        Ok(result) => result,
        Err(err) => {
            error!(environment_id=%env.id, error=?err, "Failed listing websites");
            return;
        }
    };

    summary.record_endpoint(
        websites_result.ssphp_collection_outcome,
        event_count(&websites_result),
    );

    let websites: Vec<WebsiteDto> = websites_result.websites();

    if websites.is_empty() {
        if websites_result.ssphp_http_status >= 200 && websites_result.ssphp_http_status < 300 {
            let no_sites =
                PowerPagesClient::no_power_pages_sites(env, &websites_result.metadata.request_url);
            let _ = try_collect_send(
                &format!("Power Pages no sites in env {}", env.id),
                async { Ok(no_sites) },
                splunk,
            )
            .await;
            summary.record_env_no_sites();
        }
        return;
    }

    summary.record_env_with_sites(websites.len());

    for website in websites {
        collect_website(client, splunk, env, &website, summary).await;
    }
}

async fn collect_website(
    client: &PowerPagesClient,
    splunk: &Splunk,
    env: &EnvironmentResponse,
    website: &WebsiteDto,
    summary: &mut RunSummary,
) {
    let site_label = format!("{}/{}", env.id, website.id);

    let hostnames_result = collect(
        splunk,
        summary,
        &format!("Power Pages hostnames for {site_label}"),
        client.get_hostnames(env, website),
    )
    .await;
    let mut hostnames = hostnames_result
        .as_ref()
        .map(PowerPagesApiResult::hostnames)
        .unwrap_or_default();
    if hostnames.is_empty() {
        hostnames = website.custom_host_names.clone();
    }
    if hostnames.is_empty() {
        if let Some(subdomain) = website.subdomain.as_ref().filter(|s| !s.is_empty()) {
            hostnames.push(subdomain.clone());
        }
    }

    let waf_status_result = collect(
        splunk,
        summary,
        &format!("Power Pages WAF status for {site_label}"),
        client.get_waf_status(env, website),
    )
    .await;
    let waf_active = waf_status_result
        .as_ref()
        .and_then(PowerPagesApiResult::waf_status)
        .map(|s| s == "Created")
        .unwrap_or(false);

    let _ = collect(
        splunk,
        summary,
        &format!("Power Pages deep scan score for {site_label}"),
        client.get_deep_scan_score(env, website),
    )
    .await;

    let _ = collect(
        splunk,
        summary,
        &format!("Power Pages deep scan report for {site_label}"),
        client.get_deep_scan_report(env, website),
    )
    .await;

    let _ = collect(
        splunk,
        summary,
        &format!("Power Pages allowed IPs for {site_label}"),
        client.get_allowed_ips(env, website),
    )
    .await;

    let _ = collect(
        splunk,
        summary,
        &format!("Power Pages certificates SSL for {site_label}"),
        client.get_certificates(env, website, "SSL"),
    )
    .await;

    let _ = collect(
        splunk,
        summary,
        &format!("Power Pages certificates MANAGED for {site_label}"),
        client.get_certificates(env, website, "MANAGED"),
    )
    .await;

    if waf_active {
        let _ = collect(
            splunk,
            summary,
            &format!("Power Pages WAF rules for {site_label}"),
            client.get_waf_rules(env, website),
        )
        .await;
    } else {
        let inactive = PowerPagesClient::waf_inactive(
            env,
            website,
            &waf_status_result
                .as_ref()
                .and_then(PowerPagesApiResult::waf_status)
                .unwrap_or_else(|| "unknown".to_owned()),
        );
        let _ = try_collect_send(
            &format!("Power Pages WAF inactive for {site_label}"),
            async { Ok(inactive) },
            splunk,
        )
        .await;
    }

    for hostname in hostnames {
        let _ = collect(
            splunk,
            summary,
            &format!("Power Pages SSL bindings for {site_label} hostname {hostname}"),
            client.get_ssl_bindings(env, website, &hostname),
        )
        .await;
    }
}

async fn collect(
    splunk: &Splunk,
    summary: &mut RunSummary,
    name: &str,
    future: impl std::future::Future<Output = Result<PowerPagesApiResult>>,
) -> Option<PowerPagesApiResult> {
    match try_collect_send(name, future, splunk).await {
        Ok(result) => {
            summary.record_endpoint(result.ssphp_collection_outcome, event_count(&result));
            Some(result)
        }
        Err(err) => {
            error!(collection=name, error=?err, "Power Pages collection failed");
            None
        }
    }
}
