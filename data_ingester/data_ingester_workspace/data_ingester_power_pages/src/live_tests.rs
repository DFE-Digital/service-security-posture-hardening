use std::env;

use anyhow::{Context, Result};
use data_ingester_splunk::splunk::{set_ssphp_run, Splunk, SplunkTrait};
use data_ingester_supporting::keyvault::get_keyvault_secrets;

use crate::client::PowerPagesClient;
use crate::entrypoint;
use crate::response::CollectionOutcome;
use crate::SSPHP_RUN_KEY;

#[tokio::test]
async fn live_power_pages_entrypoint() -> Result<()> {
    let _ = set_ssphp_run(SSPHP_RUN_KEY);
    let secrets = get_keyvault_secrets(
        &env::var("KEY_VAULT_NAME").expect("Need KEY_VAULT_NAME environment variable"),
    )
    .await?;
    let splunk = Splunk::new(
        secrets.splunk_host.as_ref().context("No splunk_host")?,
        secrets.splunk_token.as_ref().context("No splunk_token")?,
        true,
    )?;
    entrypoint(std::sync::Arc::new(secrets), std::sync::Arc::new(splunk)).await
}

/// Narrow live probe: authenticates against Entra, hits the Power Platform
/// environments endpoint, and for each environment hits the Power Pages
/// `websites` endpoint. Does NOT send anything to Splunk or run per-site
/// scans (WAF, deep scan, certificates, SSL bindings, etc.).
///
/// Requires KEY_VAULT_NAME env var; reads azure_client_id / azure_client_secret
/// / azure_tenant_id secrets from Key Vault.
#[tokio::test]
async fn live_power_pages_website_endpoint_access() -> Result<()> {
    let _ = set_ssphp_run(SSPHP_RUN_KEY);
    let secrets = get_keyvault_secrets(
        &env::var("KEY_VAULT_NAME").expect("Need KEY_VAULT_NAME environment variable"),
    )
    .await?;

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

    let env_result = client
        .list_environments()
        .await
        .context("Listing Power Platform environments")?;

    eprintln!(
        "environments endpoint: status={} outcome={:?} url={}",
        env_result.ssphp_http_status,
        env_result.ssphp_collection_outcome,
        env_result.metadata.request_url,
    );

    assert!(
        env_result.ssphp_collection_outcome != CollectionOutcome::PreflightFailed,
        "Pre-flight against environments endpoint failed with HTTP {}: {:?}",
        env_result.ssphp_http_status,
        env_result.error_message,
    );

    let environments = env_result.environments();
    assert!(
        !environments.is_empty(),
        "No environments returned from Power Platform API — check RBAC / admin app registration"
    );

    let mut sites_seen = 0usize;
    for env in &environments {
        let websites_result = client
            .list_websites(env)
            .await
            .with_context(|| format!("Listing websites for environment {}", env.id))?;

        eprintln!(
            "  websites endpoint env={} name={:?} status={} outcome={:?}",
            env.id,
            env.display_name,
            websites_result.ssphp_http_status,
            websites_result.ssphp_collection_outcome,
        );

        assert!(
            (200..300).contains(&websites_result.ssphp_http_status)
                || websites_result.ssphp_http_status == 404,
            "Unexpected HTTP {} from websites endpoint for env {}: {:?}",
            websites_result.ssphp_http_status,
            env.id,
            websites_result.error_message,
        );

        for site in websites_result.websites() {
            sites_seen += 1;
            eprintln!(
                "    site id={} name={} url={} subdomain={:?} custom_hosts={:?}",
                site.id, site.name, site.website_url, site.subdomain, site.custom_host_names,
            );
        }
    }

    eprintln!(
        "Total environments probed: {} — Power Pages sites discovered: {}",
        environments.len(),
        sites_seen
    );

    Ok(())
}

/// Query Entra (Microsoft Graph) for the RBAC/permission picture of the
/// service principal used by the Power Pages ingester:
///   - the service principal object (appId, displayName, accountEnabled)
///   - application permissions granted to it (`appRoleAssignments`)
///   - delegated permissions granted to it (`oauth2PermissionGrants`)
///   - directory role & group memberships (`memberOf`)
///
/// Prints the results and asserts that the token can be acquired and the
/// SP is resolvable. Does NOT touch Splunk or Power Platform.
///
/// Requires KEY_VAULT_NAME. Requires the SP to have at minimum
/// `Application.Read.All` (or `Directory.Read.All`) on Microsoft Graph —
/// if it doesn't, the appRoleAssignments/memberOf calls will 403, which
/// is itself useful diagnostic output.
#[tokio::test]
async fn live_entra_rbac_permissions() -> Result<()> {
    use reqwest::Client;
    use serde_json::Value;

    let _ = set_ssphp_run(SSPHP_RUN_KEY);
    let secrets = get_keyvault_secrets(
        &env::var("KEY_VAULT_NAME").expect("Need KEY_VAULT_NAME environment variable"),
    )
    .await?;

    let client_id = secrets
        .azure_client_id
        .as_ref()
        .context("Expect azure_client_id secret")?
        .to_owned();
    let client_secret = secrets
        .azure_client_secret
        .as_ref()
        .context("Expect azure_client_secret secret")?
        .to_owned();
    let tenant_id = secrets
        .azure_tenant_id
        .as_ref()
        .context("Expect azure_tenant_id secret")?
        .to_owned();

    let http = Client::new();

    // 1. Acquire a Microsoft Graph token via client credentials.
    let token_url =
        format!("https://login.microsoftonline.com/{tenant_id}/oauth2/v2.0/token");
    let token_resp: Value = http
        .post(&token_url)
        .form(&[
            ("client_id", client_id.as_str()),
            ("client_secret", client_secret.as_str()),
            ("grant_type", "client_credentials"),
            ("scope", "https://graph.microsoft.com/.default"),
        ])
        .send()
        .await
        .context("Requesting Graph token")?
        .json()
        .await
        .context("Parsing Graph token JSON")?;
    let token = token_resp
        .get("access_token")
        .and_then(|v| v.as_str())
        .with_context(|| format!("No access_token in Graph token response: {token_resp}"))?
        .to_owned();

    // Helper: GET a Graph URL, return (status, json body).
    let graph_get = |url: String, token: String| {
        let http = http.clone();
        async move {
            let resp = http
                .get(&url)
                .bearer_auth(&token)
                .send()
                .await
                .with_context(|| format!("GET {url}"))?;
            let status = resp.status().as_u16();
            let body: Value = resp
                .json()
                .await
                .unwrap_or_else(|e| serde_json::json!({ "parse_error": e.to_string() }));
            Ok::<(u16, Value), anyhow::Error>((status, body))
        }
    };

    // 2. Resolve the service principal by appId.
    let sp_url = format!(
        "https://graph.microsoft.com/v1.0/servicePrincipals(appId='{client_id}')"
    );
    let (sp_status, sp_body) = graph_get(sp_url.clone(), token.clone()).await?;
    eprintln!("servicePrincipals(appId='...'): HTTP {sp_status}");
    eprintln!("{}", serde_json::to_string_pretty(&sp_body)?);
    assert!(
        (200..300).contains(&sp_status),
        "Could not resolve service principal via Graph (HTTP {sp_status}): {sp_body}"
    );
    let sp_object_id = sp_body
        .get("id")
        .and_then(|v| v.as_str())
        .context("Service principal response missing id")?
        .to_owned();
    eprintln!(
        "  displayName={:?} accountEnabled={:?} appId={:?} objectId={}",
        sp_body.get("displayName"),
        sp_body.get("accountEnabled"),
        sp_body.get("appId"),
        sp_object_id,
    );

    // 3. Application permissions granted TO this SP (appRoleAssignments).
    let (ara_status, ara_body) = graph_get(
        format!(
            "https://graph.microsoft.com/v1.0/servicePrincipals/{sp_object_id}/appRoleAssignments"
        ),
        token.clone(),
    )
    .await?;
    eprintln!("\nappRoleAssignments (application permissions granted): HTTP {ara_status}");
    if let Some(items) = ara_body.get("value").and_then(|v| v.as_array()) {
        eprintln!("  count={}", items.len());
        for item in items {
            eprintln!(
                "  - resource={:?} appRoleId={:?} principalId={:?} createdDateTime={:?}",
                item.get("resourceDisplayName"),
                item.get("appRoleId"),
                item.get("principalId"),
                item.get("createdDateTime"),
            );
        }
    } else {
        eprintln!("{}", serde_json::to_string_pretty(&ara_body)?);
    }

    // 4. Delegated permission grants (oauth2PermissionGrants).
    let (o2_status, o2_body) = graph_get(
        format!(
            "https://graph.microsoft.com/v1.0/servicePrincipals/{sp_object_id}/oauth2PermissionGrants"
        ),
        token.clone(),
    )
    .await?;
    eprintln!("\noauth2PermissionGrants (delegated permissions): HTTP {o2_status}");
    if let Some(items) = o2_body.get("value").and_then(|v| v.as_array()) {
        eprintln!("  count={}", items.len());
        for item in items {
            eprintln!(
                "  - clientId={:?} resourceId={:?} scope={:?} consentType={:?}",
                item.get("clientId"),
                item.get("resourceId"),
                item.get("scope"),
                item.get("consentType"),
            );
        }
    } else {
        eprintln!("{}", serde_json::to_string_pretty(&o2_body)?);
    }

    // 5. Directory roles & group memberships.
    let (mo_status, mo_body) = graph_get(
        format!(
            "https://graph.microsoft.com/v1.0/servicePrincipals/{sp_object_id}/memberOf"
        ),
        token.clone(),
    )
    .await?;
    eprintln!("\nmemberOf (directory roles & groups): HTTP {mo_status}");
    if let Some(items) = mo_body.get("value").and_then(|v| v.as_array()) {
        eprintln!("  count={}", items.len());
        for item in items {
            eprintln!(
                "  - @odata.type={:?} displayName={:?} id={:?} roleTemplateId={:?}",
                item.get("@odata.type"),
                item.get("displayName"),
                item.get("id"),
                item.get("roleTemplateId"),
            );
        }
    } else {
        eprintln!("{}", serde_json::to_string_pretty(&mo_body)?);
    }

    // 6. Human-friendly summary of statuses.
    eprintln!(
        "\nEntra RBAC probe summary: sp={sp_status} appRoleAssignments={ara_status} \
         oauth2PermissionGrants={o2_status} memberOf={mo_status}"
    );

    Ok(())
}
