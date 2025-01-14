use anyhow::{Result, anyhow};
use data_ingester_splunk::splunk::ToHecEvents;
use itertools::Itertools;
use reqwest::header::HeaderMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::error;

/// https://learn.microsoft.com/en-us/azure/devops/integrate/concepts/rate-limits?view=azure-devops
#[derive(Debug, Deserialize)]
pub(crate) struct AdoRateLimiting {
    #[allow(unused)]
    #[serde(rename = "Retry-After")]
    retry_after: usize,

    #[allow(unused)]
    #[serde(rename = "X-RateLimit-Resource")]
    rate_limit_resource: String,

    #[allow(unused)]
    #[serde(rename = "X-RateLimit-Delay")]
    rate_limit_delay: usize,

    #[allow(unused)]
    #[serde(rename = "X-RateLimit-Limit")]
    rate_limit_limit: usize,

    #[allow(unused)]
    #[serde(rename = "X-RateLimit-Remaining")]
    rate_limit_remaining: usize,

    #[allow(unused)]
    #[serde(rename = "X-RateLimit-Reset")]
    // Uknown type
    rate_limit_reset: usize,
}

impl AdoRateLimiting {
    fn get_usize_from_header(headers: &HeaderMap, key: &str) -> usize {
        static DEFAULT: usize = 0;
        headers
            .get(key)
            .map(|h| {
                h.to_str()
                    .unwrap_or_default()
                    .parse::<usize>()
                    .unwrap_or(DEFAULT)
            })
            .unwrap_or(DEFAULT)
    }

    pub(crate) fn from_headers(headers: &HeaderMap) -> Self {
        error!("Ado response headers: {:?}", headers);

        let rate_limit_resource = headers
            .get("X-RateLimit-Resource")
            .and_then(|header_value| header_value.to_str().ok())
            .unwrap_or("SSPHP:Unknown Resource")
            .to_string();
        let limits = Self {
            retry_after: Self::get_usize_from_header(headers, "Retry-After"),
            rate_limit_resource,
            rate_limit_delay: Self::get_usize_from_header(headers, "X-RateLimit-Delay"),
            rate_limit_limit: Self::get_usize_from_header(headers, "X-RateLimit-Limit"),
            rate_limit_remaining: Self::get_usize_from_header(headers, "X-RateLimit-Remaining"),
            rate_limit_reset: Self::get_usize_from_header(headers, "X-RateLimit-Reset"),
        };
        error!("Ado parsed limits: {:?}", limits);
        if headers.contains_key("Retry-After") || headers.contains_key("retry-after") {
            unreachable!("PLEASE DEBUG RETRY HEADERS")
        }
        limits
    }
}

#[allow(unused)]
struct AdoPaging {
    #[allow(unused)]
    continuation_token: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct AdoMetadata {
    url: String,
    organization: Option<String>,
    pub(crate) status: u16,
    pub(crate) source: String,
    pub(crate) sourcetype: String,
    tenant: String,
    r#type: String,
    rest_docs: String,
}

impl AdoMetadata {
    pub(crate) fn new(
        tenant: &str,
        url: &str,
        organization: Option<&str>,
        status: u16,
        r#type: &str,
        rest_docs: &str,
    ) -> Self {
        Self {
            r#type: r#type.into(),
            tenant: tenant.into(),
            source: format!("{}:{}", tenant, url),
            url: url.into(),
            organization: organization.map(|o| o.into()),
            status,
            sourcetype: "ADO".into(),
            rest_docs: rest_docs.into(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct AdoResponse {
    pub(crate) count: usize,
    pub(crate) value: Vec<Value>,
    #[serde(skip_deserializing, default, flatten)]
    pub(crate) metadata: Option<AdoMetadata>,
}

impl ToHecEvents for AdoResponse {
    type Item = Value;

    fn source(&self) -> &str {
        self.metadata
            .as_ref()
            .map(|metadata| metadata.source.as_str())
            .unwrap_or("NO ADOMETADATA FOR SOURCE")
    }

    fn sourcetype(&self) -> &str {
        self.metadata
            .as_ref()
            .map(|metadata| metadata.sourcetype.as_str())
            .unwrap_or("NO ADOMETADATA FOR SOURCETYPE")
    }

    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        Box::new(self.value.iter())
    }

    fn ssphp_run_key(&self) -> &str {
        "azure_devops"
    }

    fn to_hec_events(&self) -> Result<Vec<data_ingester_splunk::splunk::HecEvent>> {
        let (ok, err): (Vec<_>, Vec<_>) = self
            .collection()
            .map(|ado_response| {
                let mut ado_response = ado_response.clone();
                let ssphp_debug = if let Some(metadata) = &self.metadata {
                    serde_json::to_value(metadata).unwrap_or_else(|_| {
                        serde_json::to_value("Error Getting AdoMetadata")
                            .expect("Value from static str should not fail")
                    })
                } else {
                    serde_json::to_value("No AdoMetadata")
                        .expect("Value from static str should not fail")
                };

                let _ = ado_response
                    .as_object_mut()
                    .expect("ado_response should always be accessible as an Value object")
                    .insert("SSPHP_DEBUG".into(), ssphp_debug);
                data_ingester_splunk::splunk::HecEvent::new_with_ssphp_run(
                    &ado_response,
                    self.source(),
                    self.sourcetype(),
                    self.get_ssphp_run(),
                )
            })
            .partition_result();
        if !err.is_empty() {
            return Err(anyhow!(
                err.iter()
                    .map(|err| format!("{:?}", err))
                    .collect::<Vec<String>>()
                    .join("\n")
            ));
        }
        Ok(ok)
    }
}

impl ToHecEvents for &AdoResponse {
    type Item = Value;

    fn source(&self) -> &str {
        self.metadata
            .as_ref()
            .map(|metadata| metadata.source.as_str())
            .unwrap_or("NO ADOMETADATA FOR SOURCE")
    }

    fn sourcetype(&self) -> &str {
        self.metadata
            .as_ref()
            .map(|metadata| metadata.sourcetype.as_str())
            .unwrap_or("NO ADOMETADATA FOR SOURCETYPE")
    }

    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        Box::new(self.value.iter())
    }

    fn ssphp_run_key(&self) -> &str {
        "azure_devops"
    }

    fn to_hec_events(&self) -> Result<Vec<data_ingester_splunk::splunk::HecEvent>> {
        let (ok, err): (Vec<_>, Vec<_>) = self
            .collection()
            .map(|ado_response| {
                let mut ado_response = ado_response.clone();
                let ssphp_debug = if let Some(metadata) = &self.metadata {
                    serde_json::to_value(metadata).unwrap_or_else(|_| {
                        serde_json::to_value("Error Getting AdoMetadata")
                            .expect("Value from static str should not fail")
                    })
                } else {
                    serde_json::to_value("No AdoMetadata")
                        .expect("Value from static str should not fail")
                };

                let _ = ado_response
                    .as_object_mut()
                    .expect("ado_response should always be accessible as an Value object")
                    .insert("SSPHP_DEBUG".into(), ssphp_debug);
                data_ingester_splunk::splunk::HecEvent::new_with_ssphp_run(
                    &ado_response,
                    self.source(),
                    self.sourcetype(),
                    self.get_ssphp_run(),
                )
            })
            .partition_result();
        if !err.is_empty() {
            return Err(anyhow!(
                err.iter()
                    .map(|err| format!("{:?}", err))
                    .collect::<Vec<String>>()
                    .join("\n")
            ));
        }
        Ok(ok)
    }
}
