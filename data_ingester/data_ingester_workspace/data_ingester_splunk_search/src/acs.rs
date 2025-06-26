use anyhow::{Context, Result};
use reqwest::{
    header::{HeaderMap, HeaderValue},
    Client,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::time::Duration;
use tracing::{debug, error, info};

/// A simple client for Splunk ACS
/// https://docs.splunk.com/Documentation/SplunkCloud/9.1.2312/Config/ACSIntro
#[derive(Debug, Default)]
pub struct Acs {
    client: Client,
    stack: String,
    current_cidr: Option<String>,
}

/// Represents an ACS IpAllowList
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct IpAllowList {
    subnets: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum IpAllowListResponse {
    Valid(IpAllowList),
    Invalid(Value),
}

impl IpAllowList {
    fn format_cidr(ip: &str, range: usize) -> String {
        format!("{}/{}", ip, range)
    }
}

/// IP response from Ipify
#[derive(Debug, Deserialize, Default)]
struct Ipify {
    ip: String,
}

impl Acs {
    /// Create a new ACS client for a given 'stack' using 'token' to authenticate
    pub fn new(stack: &str, token: &str) -> Result<Self> {
        let client = reqwest::ClientBuilder::new()
            .danger_accept_invalid_certs(false)
            .default_headers(Acs::headers(token)?)
            .build()?;
        debug!("ACS Client: {:?}", &client);
        debug!("Splunk ACS Stack: {:?}", stack);
        Ok(Self {
            stack: stack.to_string(),
            client,
            current_cidr: None,
        })
    }

    /// Default headers for all ACS requests
    fn headers(token: &str) -> Result<HeaderMap> {
        let mut headers = HeaderMap::new();

        let mut auth = HeaderValue::from_str(&format!("Bearer {}", token))?;
        auth.set_sensitive(true);
        _ = headers.insert("Authorization", auth);

        // ACS rejects requests without a user agent
        let user_agent = HeaderValue::from_str("curl/8.4.0")?;
        _ = headers.insert("User-Agent", user_agent);

        let content_type = HeaderValue::from_str("application/json")?;
        _ = headers.insert("Content-Type", content_type);
        Ok(headers)
    }

    /// List CIDRs allowed to access the search-api REST endpoint
    pub async fn list_search_api_ip_allow_list(&self) -> Result<IpAllowList> {
        info!("ACS: Getting 'search-api' ip_allow_list");
        let url = format!(
            "https://admin.splunk.com/{}/adminconfig/v2/access/{}/ipallowlists",
            self.stack, "search-api"
        );
        let request = self
            .client
            .get(url)
            .build()
            .context("Build request for ACS list IP Allow list")?;
        debug!("Acs request: {:?}", &request);

        let response = self
            .client
            .execute(request)
            .await
            .context("Sending request for ACS list IP allow list")?;

        let status = response.status();

        if !status.is_success() {
            let headers = format!("{:?}", &response);
            let body = response
                .text()
                .await
                .context("Failed to get failed response body")?;
            anyhow::bail!(
                "Failed to request IpAllowList list\n\n{:?}\n{:?}\n{:?}",
                self.acs_error_codes(status.into()),
                headers,
                body
            );
        }

        let ip_allow_list: IpAllowListResponse = response
            .json()
            .await
            .context("Parsing ACS List IP response as IpAllowList")?;

        let ip_allow_list = match ip_allow_list {
            IpAllowListResponse::Valid(ip_allow_list) => ip_allow_list,
            IpAllowListResponse::Invalid(invalid) => {
                error!(invalid_response=?invalid, "Error while Decoding ACS response");
                anyhow::bail!("Error while Decoding ACS response: {}", invalid);
            }
        };

        Ok(ip_allow_list)
    }

    /// Add a CIDR to the search-api IP allow list
    pub async fn add_search_api_ip_allow_list(&self, cidr: &str) -> Result<()> {
        info!("ACS: Adding IP:'{}' to 'search-api' ip_allow_list", cidr);
        let url = format!(
            "https://admin.splunk.com/{}/adminconfig/v2/access/{}/ipallowlists",
            self.stack, "search-api"
        );
        let ip_allow_list = IpAllowList {
            subnets: vec![cidr.to_string()],
        };
        let request = self
            .client
            .post(url)
            .json(&ip_allow_list)
            .build()
            .context("Build request for ACS set IP Allow list")?;
        debug!("request: {:?}", &request);
        let response = self
            .client
            .execute(request)
            .await
            .context("Sending request for ACS set IP allow list")?;
        if !response.status().is_success() {
            let headers = format!("{:?}", &response);
            let body = response
                .text()
                .await
                .context("Failed to get failed response body")?;
            anyhow::bail!(
                "Failed to add '{}' to ACS search-api ip allow list\n{:?}\n{:?}",
                cidr,
                headers,
                body
            );
        }
        Ok(())
    }

    pub async fn remove_current_cidr(&mut self) -> Result<()> {
        if let Some(cidr) = self.current_cidr.as_ref() {
            self.delete_search_api_ip_allow_list(cidr)
                .await
                .context("ACS: Removing current_ip from search-api ip_allow_list")?;
        } else {
            let ip = self.get_current_ip().await?;
            let cidr = format!("{}/32", ip);
            self.delete_search_api_ip_allow_list(&cidr)
                .await
                .context("ACS: Removing current_ip from search-api ip_allow_list")?;
        }
        Ok(())
    }

    pub async fn delete_search_api_ip_allow_list(&self, cidr: &str) -> Result<()> {
        info!(
            "ACS: Deleting IP:'{}' from 'search-api' ip_allow_list",
            cidr
        );
        let url = format!(
            "https://admin.splunk.com/{}/adminconfig/v2/access/{}/ipallowlists",
            self.stack, "search-api"
        );
        debug!(url);
        let ip_allow_list = IpAllowList {
            subnets: vec![cidr.to_string()],
        };
        let request = self
            .client
            .delete(url)
            .json(&ip_allow_list)
            .build()
            .context("Build request for ACS delete IP Allow list")?;
        let response = self
            .client
            .execute(request)
            .await
            .context("Sending request for ACS delete IP allow list")?;
        if !response.status().is_success() {
            let headers = format!("{:?}", &response);
            let body = response
                .text()
                .await
                .context("Failed to get failed response body")?;
            anyhow::bail!(
                "Failed to delete '{}' to ACS search-api ip allow list\n{:?}\n{:?}",
                cidr,
                headers,
                body
            );
        }
        Ok(())
    }

    pub async fn get_current_ip(&self) -> Result<String> {
        let url = "https://api.ipify.org?format=json";
        let ipify = reqwest::get(url)
            .await
            .context("Sending Request to Ipify")?
            .json::<Ipify>()
            .await
            .context("Parsing Ipify JSON response")?;
        let url = "https://ifconfig.me/ip";
        let ifconfig_me = reqwest::get(url)
            .await
            .context("Sending Request to ifconfig.me")?
            .text()
            .await
            .context("Getting body from ifconfig.me")?;
        if ipify.ip != ifconfig_me {
            let message = format!(
                "Deteceted IPs don't match ipify:{} ipconfig.me:{}",
                ipify.ip, ifconfig_me
            );
            anyhow::bail!(message);
        }
        Ok(ipify.ip)
    }

    /// TODO: Poll for status https://admin.splunk.com/{stack}/adminconfig/v2/status
    /// https://docs.splunk.com/Documentation/SplunkCloud/9.1.2312/Config/ConfigureIPAllowList
    pub async fn wait_for_ip_allow_list_update(&self) -> Result<()> {
        info!("ACS Waiting for ip allow list to update...");
        let now = tokio::time::Instant::now();
        let url = format!("https://{}.splunkcloud.com:8089/", self.stack);
        const MAX_WAIT_TIME: u64 = 60 * 10;
        loop {
            match reqwest::Client::new()
                .get(&url)
                .timeout(Duration::from_secs(5))
                .send()
                .await
            {
                Ok(_) => break,
                Err(_) => {
                    info!("Waiting for port update");
                    tokio::time::sleep(Duration::from_secs(50)).await;
                }
            }

            if now.elapsed().as_secs() > MAX_WAIT_TIME {
                anyhow::bail!("Waited too long for IP Allow list update");
            }
        }
        let elapsed = now.elapsed().as_secs_f32();
        info!(
            "ACS ip:'{:?}' added to search-api ip allow list in {} seconds",
            &self.current_cidr,
            elapsed.to_string()
        );
        Ok(())
    }

    pub async fn grant_access_for_current_ip(&mut self) -> Result<()> {
        let current_ip = self.get_current_ip().await.context("Get current IP")?;
        let allow_list = self
            .list_search_api_ip_allow_list()
            .await
            .context("Listing Allowed IPs")?;

        let current_cidr = IpAllowList::format_cidr(&current_ip, 32);
        self.current_cidr = Some(current_cidr.to_string());

        if allow_list.subnets.contains(&current_cidr) {
            info!("Current Cidr already in IP allow list");
            return Ok(());
        }

        self.add_search_api_ip_allow_list(
            self.current_cidr
                .as_ref()
                .expect("just set self.current_ip"),
        )
        .await
        .context("Add 'current_ip' to search_api IP allow list")?;

        self.wait_for_ip_allow_list_update()
            .await
            .context("Waiting for ip allow list to update")?;
        Ok(())
    }

    fn acs_error_codes(&self, error: u16) -> &str {
        match error {
            200 => "Request completed successfully.",
            201 => "Create request completed successfully.",
            202 => "Request accepted for processing, but processing has not completed.",
            400 => "Request error. See response body for details.",
            401 => "Authentication failure, invalid access credentials.",
            402 => "In-use Splunk software license disables this feature.",
            403 => "Insufficient permission.",
            404 => "Requested endpoint does not exist.",
            409 => "Object already exists.",
            500 => "Unspecified internal server error. See response body for details.",
            503 => "Service Temporarily Unavailable, Please Try again.",
            _ => "Unknown HTTP status code",
        }
    }
}

#[cfg(test)]
mod test {
    use anyhow::{Context, Result};

    use super::Acs;

    #[tokio::test]
    async fn test_client_new() -> Result<()> {
        let _ = Acs::new("foo", "tokenbar");
        Ok(())
    }

    #[tokio::test]
    async fn test_client_get_current_ip() -> Result<()> {
        let acs = Acs::new("foo", "tokenbar")?;
        let ip = acs.get_current_ip().await.context("Testing current IP")?;
        assert!(ip.contains('.'));
        Ok(())
    }

    #[cfg(feature = "live_tests")]
    #[tokio::test]
    async fn test_client_add_ip_to_allow_list() -> Result<()> {
        let acs = Acs::new("foo", "tokenbar")?;
        acs.add_search_api_ip_allow_list("1.2.3.4").await?;
        Ok(())
    }

    #[cfg(feature = "live_tests")]
    #[tokio::test]
    async fn test_client_grant_access_for_current_ip() -> Result<()> {
        let mut acs = Acs::new("foo", "tokenbar")?;
        acs.grant_access_for_current_ip().await?;
        Ok(())
    }
}
