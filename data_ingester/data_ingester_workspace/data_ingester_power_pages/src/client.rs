use anyhow::{Context, Result};
use reqwest::Client;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::metadata::{DataType, PowerPagesMetadata};
use crate::models::{EnvironmentResponse, WebsiteDto};
use crate::response::{http_status_success, CollectionOutcome, PowerPagesApiResult};

pub const API_VERSION: &str = "2024-10-01";
const DEFAULT_API_BASE: &str = "https://api.powerplatform.com";
const OAUTH_SCOPE: &str = "https://api.powerplatform.com/.default";

pub struct PowerPagesClient {
    client: Client,
    token: String,
    api_base: String,
}

impl PowerPagesClient {
    pub async fn new(client_id: &str, client_secret: &str, tenant_id: &str) -> Result<Self> {
        let client = Client::new();
        let url = format!("https://login.microsoftonline.com/{tenant_id}/oauth2/v2.0/token");
        let params = [
            ("client_id", client_id),
            ("client_secret", client_secret),
            ("grant_type", "client_credentials"),
            ("scope", OAUTH_SCOPE),
        ];
        let response = client
            .post(url)
            .form(&params)
            .send()
            .await
            .context("Power Pages OAuth token request")?;

        let token: TokenResponse = response
            .json()
            .await
            .context("Parsing Power Pages OAuth token response")?;

        Ok(Self {
            client,
            token: token.access_token,
            api_base: DEFAULT_API_BASE.to_owned(),
        })
    }

    #[cfg(test)]
    pub fn new_for_test(api_base: &str, token: &str) -> Self {
        Self {
            client: Client::new(),
            token: token.to_owned(),
            api_base: api_base.to_owned(),
        }
    }

    pub async fn list_environments(&self) -> Result<PowerPagesApiResult> {
        let url = format!(
            "{}/environmentmanagement/environments?api-version={API_VERSION}",
            self.api_base
        );
        let metadata = PowerPagesMetadata {
            data_type: DataType::Preflight,
            environment_id: None,
            environment_name: None,
            website_id: None,
            website_name: None,
            website_url: None,
            request_url: url.clone(),
            hostname: None,
            cert_type: None,
            is_deep_scan: false,
        };
        self.get_paginated::<EnvironmentResponse>(&url, metadata, "value")
            .await
    }

    pub async fn list_websites(&self, env: &EnvironmentResponse) -> Result<PowerPagesApiResult> {
        let url = format!(
            "{}/powerpages/environments/{}/websites?api-version={API_VERSION}",
            self.api_base, env.id
        );
        let metadata = PowerPagesMetadata {
            data_type: DataType::Website,
            environment_id: Some(env.id.clone()),
            environment_name: Some(env.display_name.clone()),
            website_id: None,
            website_name: None,
            website_url: None,
            request_url: url.clone(),
            hostname: None,
            cert_type: None,
            is_deep_scan: false,
        };
        self.get_paginated::<WebsiteDto>(&url, metadata, "value")
            .await
    }

    pub async fn get_hostnames(
        &self,
        env: &EnvironmentResponse,
        website: &WebsiteDto,
    ) -> Result<PowerPagesApiResult> {
        let url = format!(
            "{}/powerpages/environments/{}/websites/{}/customDomain?api-version={API_VERSION}",
            self.api_base, env.id, website.id
        );
        let metadata = PowerPagesMetadata::for_website(env, website, DataType::Hostname, &url);
        self.get_json_array(&url, metadata).await
    }

    pub async fn get_ssl_bindings(
        &self,
        env: &EnvironmentResponse,
        website: &WebsiteDto,
        hostname: &str,
    ) -> Result<PowerPagesApiResult> {
        let url = format!(
            "{}/powerpages/environments/{}/websites/{}/sslBindings?hostName={}&api-version={API_VERSION}",
            self.api_base, env.id, website.id, hostname
        );
        let mut metadata =
            PowerPagesMetadata::for_website(env, website, DataType::SslBinding, &url);
        metadata.hostname = Some(hostname.to_owned());
        self.get_json_array(&url, metadata).await
    }

    pub async fn get_deep_scan_report(
        &self,
        env: &EnvironmentResponse,
        website: &WebsiteDto,
    ) -> Result<PowerPagesApiResult> {
        let url = format!(
            "{}/powerpages/environments/{}/websites/{}/scan/deep/getLatestCompletedReport?api-version={API_VERSION}",
            self.api_base, env.id, website.id
        );
        let mut metadata =
            PowerPagesMetadata::for_website(env, website, DataType::DeepScanReport, &url);
        metadata.is_deep_scan = true;
        self.get(&url, metadata).await
    }

    pub async fn get_deep_scan_score(
        &self,
        env: &EnvironmentResponse,
        website: &WebsiteDto,
    ) -> Result<PowerPagesApiResult> {
        let url = format!(
            "{}/powerpages/environments/{}/websites/{}/scan/deep/getSecurityScore?api-version={API_VERSION}",
            self.api_base, env.id, website.id
        );
        let mut metadata =
            PowerPagesMetadata::for_website(env, website, DataType::DeepScanScore, &url);
        metadata.is_deep_scan = true;
        self.get(&url, metadata).await
    }

    pub async fn get_waf_status(
        &self,
        env: &EnvironmentResponse,
        website: &WebsiteDto,
    ) -> Result<PowerPagesApiResult> {
        let url = format!(
            "{}/powerpages/environments/{}/websites/{}/getWafStatus?api-version={API_VERSION}",
            self.api_base, env.id, website.id
        );
        let metadata = PowerPagesMetadata::for_website(env, website, DataType::WafStatus, &url);
        self.get(&url, metadata).await
    }

    pub async fn get_waf_rules(
        &self,
        env: &EnvironmentResponse,
        website: &WebsiteDto,
    ) -> Result<PowerPagesApiResult> {
        let url = format!(
            "{}/powerpages/environments/{}/websites/{}/getWafRules?api-version={API_VERSION}",
            self.api_base, env.id, website.id
        );
        let metadata = PowerPagesMetadata::for_website(env, website, DataType::WafCustomRule, &url);
        self.get(&url, metadata).await
    }

    pub async fn get_allowed_ips(
        &self,
        env: &EnvironmentResponse,
        website: &WebsiteDto,
    ) -> Result<PowerPagesApiResult> {
        let url = format!(
            "{}/powerpages/environments/{}/websites/{}/ipaddressrules?api-version={API_VERSION}",
            self.api_base, env.id, website.id
        );
        let metadata = PowerPagesMetadata::for_website(env, website, DataType::AllowedIp, &url);
        self.get_json_array(&url, metadata).await
    }

    pub async fn get_certificates(
        &self,
        env: &EnvironmentResponse,
        website: &WebsiteDto,
        cert_type: &str,
    ) -> Result<PowerPagesApiResult> {
        let url = format!(
            "{}/powerpages/environments/{}/websites/{}/certificates?certType={cert_type}&api-version={API_VERSION}",
            self.api_base, env.id, website.id
        );
        let mut metadata =
            PowerPagesMetadata::for_website(env, website, DataType::Certificate, &url);
        metadata.cert_type = Some(cert_type.to_owned());
        self.get_json_array(&url, metadata).await
    }

    pub fn no_power_pages_sites(
        env: &EnvironmentResponse,
        request_url: &str,
    ) -> PowerPagesApiResult {
        PowerPagesApiResult {
            ssphp_http_status: 200,
            ssphp_collection_outcome: CollectionOutcome::NoPowerPagesSites,
            response_body: None,
            error_message: None,
            data: Some(serde_json::json!({
                "message": "No Power Pages websites in environment",
                "environment_id": env.id,
                "environment_name": env.display_name,
            })),
            metadata: PowerPagesMetadata {
                data_type: DataType::NoPowerPagesSites,
                environment_id: Some(env.id.clone()),
                environment_name: Some(env.display_name.clone()),
                website_id: None,
                website_name: None,
                website_url: None,
                request_url: request_url.to_owned(),
                hostname: None,
                cert_type: None,
                is_deep_scan: false,
            },
        }
    }

    pub fn waf_inactive(
        env: &EnvironmentResponse,
        website: &WebsiteDto,
        status: &str,
    ) -> PowerPagesApiResult {
        PowerPagesApiResult {
            ssphp_http_status: 200,
            ssphp_collection_outcome: CollectionOutcome::NoData,
            response_body: None,
            error_message: None,
            data: Some(serde_json::json!({ "waf_status": status, "waf_active": false })),
            metadata: PowerPagesMetadata::for_website(
                env,
                website,
                DataType::WafInactive,
                "skipped:getWafRules",
            ),
        }
    }

    async fn get(&self, url: &str, metadata: PowerPagesMetadata) -> Result<PowerPagesApiResult> {
        let response = self
            .client
            .get(url)
            .bearer_auth(&self.token)
            .send()
            .await
            .with_context(|| format!("GET {url}"))?;
        let status = response.status().as_u16();
        let body = response
            .text()
            .await
            .with_context(|| format!("GET body {url}"))?;
        let parsed = if http_status_success(status) {
            serde_json::from_str(&body).ok()
        } else {
            None
        };
        Ok(PowerPagesApiResult::from_http(
            status, body, metadata, parsed,
        ))
    }

    async fn get_json_array(
        &self,
        url: &str,
        metadata: PowerPagesMetadata,
    ) -> Result<PowerPagesApiResult> {
        let result = self.get(url, metadata).await?;
        Ok(result)
    }

    async fn get_paginated<T: DeserializeOwned + Serialize>(
        &self,
        url: &str,
        metadata: PowerPagesMetadata,
        array_key: &str,
    ) -> Result<PowerPagesApiResult> {
        let mut next_url = Some(url.to_string());
        let mut items: Vec<T> = vec![];

        while let Some(current) = next_url.take() {
            let response = self
                .client
                .get(&current)
                .bearer_auth(&self.token)
                .send()
                .await
                .with_context(|| format!("GET {current}"))?;
            let status = response.status().as_u16();
            if !http_status_success(status) {
                let body = response.text().await.unwrap_or_default();
                return Ok(PowerPagesApiResult::from_http(status, body, metadata, None));
            }
            let body = response
                .text()
                .await
                .with_context(|| format!("GET body {current}"))?;
            let page: Value = serde_json::from_str(&body)
                .with_context(|| format!("Parse JSON from {current}"))?;
            if let Some(page_items) = page.get(array_key) {
                let mut parsed: Vec<T> = serde_json::from_value(page_items.clone())
                    .with_context(|| format!("Parse {array_key} from {current}"))?;
                items.append(&mut parsed);
            }
            next_url = page
                .get("@odata.nextLink")
                .or_else(|| page.get("@odata.nextlink"))
                .and_then(|v| v.as_str())
                .map(str::to_owned);
        }

        let data = serde_json::to_value(items)?;
        Ok(PowerPagesApiResult::from_http(
            200,
            String::new(),
            metadata,
            Some(data),
        ))
    }
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
}
