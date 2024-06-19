use anyhow::Context;
use itertools::Itertools;
use reqwest::header::HeaderMap;
use reqwest::header::HeaderValue;
use reqwest::Client;
use serde::{Deserialize, Serialize};
// #[cfg(test)]
use serde_json::Value;
use std::borrow::Borrow;
use std::collections::HashMap;
use std::sync::LazyLock;
use tracing::debug;
use tracing::error;
use tracing::info;
use tracing::warn;
// #[cfg(test)]
use anyhow::{anyhow, Result};
use std::fmt::Debug;
use std::future::Future;
use std::iter;
use std::sync::RwLock;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

// Legacy, just used for tests and logs
static SSPHP_RUN: RwLock<u64> = RwLock::new(0_u64);

static SSPHP_RUN_NEW: LazyLock<RwLock<HashMap<String, u64>>> = LazyLock::new(|| {
    let mut hm = HashMap::new();
    hm.insert("default".to_string(), 0_u64);
    RwLock::new(hm)
});

#[derive(Debug, Serialize, Deserialize)]
pub struct HecEvent {
    source: String,
    sourcetype: String,
    host: String,
    event: String,
}

impl HecEvent {
    // TODO: Should return Result
    pub fn new<T: Serialize>(event: &T, source: &str, sourcetype: &str) -> Result<HecEvent> {
        let ssphp_run = *SSPHP_RUN
            .read()
            .expect("Should always be able to read SSPHP_RUN");
        let ssphp_event = SsphpEvent { ssphp_run, event };
        let hostname = hostname::get()?
            .into_string()
            .unwrap_or("NO HOSTNAME".to_owned());
        Ok(HecEvent {
            source: source.to_string(),
            sourcetype: sourcetype.to_string(),
            host: hostname,
            event: serde_json::to_string(&ssphp_event)?,
        })
    }

    pub fn new_with_ssphp_run<T: Serialize>(
        event: &T,
        source: &str,
        sourcetype: &str,
        ssphp_run: u64,
    ) -> Result<HecEvent> {
        let ssphp_event = SsphpEvent { ssphp_run, event };
        let hostname = hostname::get()?
            .into_string()
            .unwrap_or("NO HOSTNAME".to_owned());
        Ok(HecEvent {
            source: source.to_string(),
            sourcetype: sourcetype.to_string(),
            host: hostname,
            event: serde_json::to_string(&ssphp_event)?,
        })
    }
}

#[derive(Serialize, Deserialize)]
struct SsphpEvent<T> {
    #[serde(rename = "SSPHP_RUN")]
    ssphp_run: u64,
    #[serde(flatten)]
    event: T,
}

pub fn set_ssphp_run(ssphp_run_key: &str) -> Result<()> {
    let start = SystemTime::now();
    let since_the_epoch = start.duration_since(UNIX_EPOCH)?.as_secs();

    info!(
        "Setting SSPHP_RUN[{}] to {}",
        ssphp_run_key, since_the_epoch
    );
    match SSPHP_RUN_NEW.write() {
        Ok(mut ssphp_run) => {
            let _ = ssphp_run.insert(ssphp_run_key.to_string(), since_the_epoch);
        }
        Err(err) => {
            error!("Unable to lock SSPHP_RUN for writing: {:?}", err);
        }
    }
    Ok(())
}

pub fn to_hec_events<T: Serialize>(
    collection: &[T],
    source: &str,
    sourcetype: &str,
    ssphp_run_key: &str,
) -> Result<Vec<HecEvent>> {
    let ssphp_run = get_ssphp_run(ssphp_run_key);
    let (ok, err): (Vec<_>, Vec<_>) = collection
        .iter()
        .map(|u| HecEvent::new_with_ssphp_run(u, source, sourcetype, ssphp_run))
        .partition_result();
    if !err.is_empty() {
        return Err(anyhow!(err
            .iter()
            .map(|err| format!("{:?}", err))
            .collect::<Vec<String>>()
            .join("\n")));
    }
    Ok(ok)
}

pub fn get_ssphp_run(ssphp_run_key: &str) -> u64 {
    let ssphp_run = SSPHP_RUN_NEW
        .read()
        .map(|hm| {
            *hm.get(ssphp_run_key)
                .unwrap_or_else(|| hm.get("default").unwrap_or_else(|| &0))
        })
        .unwrap_or_else(|_| 0);
    ssphp_run
}

pub trait ToHecEvents {
    type Item: Serialize;
    fn to_hec_events(&self) -> Result<Vec<HecEvent>> {
        let (ok, err): (Vec<_>, Vec<_>) = self
            .collection()
            .map(|u| {
                HecEvent::new_with_ssphp_run(
                    &u,
                    self.source(),
                    self.sourcetype(),
                    self.get_ssphp_run(),
                )
            })
            .partition_result();
        if !err.is_empty() {
            return Err(anyhow!(err
                .iter()
                .map(|err| format!("{:?}", err))
                .collect::<Vec<String>>()
                .join("\n")));
        }
        Ok(ok)
    }
    fn source(&self) -> &str;
    fn sourcetype(&self) -> &str;
    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i>;
    fn get_ssphp_run(&self) -> u64 {
        let ssphp_run = SSPHP_RUN_NEW
            .read()
            .map(|hm| {
                *hm.get(self.ssphp_run_key())
                    .unwrap_or_else(|| hm.get("default").unwrap_or_else(|| &0))
            })
            .unwrap_or_else(|_| 0);
        ssphp_run
    }
    fn ssphp_run_key(&self) -> &str;
}

#[derive(Debug, Clone)]
pub struct Splunk {
    client: Client,
    url: String,
    //    client_creation_time: u64,
}

unsafe impl Send for Splunk {}
unsafe impl Sync for Splunk {}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Message {
    pub event: String,
}

impl Splunk {
    pub fn new(host: &str, token: &str) -> Result<Self> {
        let url = format!("https://{}/services/collector", host);
        let client = reqwest::ClientBuilder::new()
            .danger_accept_invalid_certs(true)
            .default_headers(Splunk::headers(token)?)
            .build()?;
        // let start = SystemTime::now();
        // let client_creation_time = start.duration_since(UNIX_EPOCH)?;
        Ok(Self { client, url })
    }

    fn headers(token: &str) -> Result<HeaderMap> {
        let mut headers = HeaderMap::new();
        let mut auth = HeaderValue::from_str(&format!("Splunk {}", token))?;
        auth.set_sensitive(true);
        _ = headers.insert("Authorization", auth);
        let channel = Uuid::new_v4().to_string();
        _ = headers.insert("X-Splunk-Request-Channel", channel.parse()?);
        Ok(headers)
    }

    #[cfg(test)]
    pub async fn send(&self, event: &HecEvent) {
        let request = self.client.post(&self.url).json(event).build().unwrap();
        let _response = self.client.execute(request).await.unwrap();
    }

    // TODO enable token acknowledgement
    pub async fn send_batch(
        &self,
        events: impl IntoIterator<Item = impl Borrow<HecEvent> + Serialize>,
    ) -> Result<()> {
        for batch in events.into_iter().batching(batch_lines) {
            let request = self
                .client
                .post(&self.url)
                .body(batch)
                .build()
                .context("building request")?;
            let _response = self
                .client
                .execute(request)
                .await
                .context("sending to splunk")?;
        }
        Ok(())
    }

    #[deprecated(note = "Use `tracing` instead.")]
    pub async fn log(&self, message: &str) -> Result<()> {
        debug!("{}", &message);
        self.send_batch(&[HecEvent::new(
            &Message {
                event: message.to_owned(),
            },
            "data_ingester_rust",
            "data_ingester_rust_logs",
        )?])
        .await?;
        Ok(())
    }
}

// Needs Splunk Creds
#[ignore]
#[tokio::test]
async fn send_to_splunk() {
    let splunk = Splunk::new("", "").unwrap();
    let data = std::collections::HashMap::from([("aktest", "fromrust")]);
    let he = HecEvent::new(&data, "msgraph_rust", "test_event").unwrap();
    splunk.send(&he).await;
}

// Needs Splunk Creds
#[ignore]
#[tokio::test]
async fn send_batch_to_splunk() {
    use std::collections::HashMap;
    let splunk = Splunk::new("", "").unwrap();
    let mut events = Vec::new();
    let data = HashMap::from([("aktest0", "fromrust")]);
    let he = HecEvent::new(&data, "msgraph_rust", "test_event").unwrap();
    events.push(he);

    let data1 = HashMap::from([("aktest1", "fromrust")]);
    let he1 = HecEvent::new(&data1, "msgraph_rust", "test_event").unwrap();
    events.push(he1);
    splunk.send_batch(&events[..]).await.unwrap();
}

fn batch_lines<I, T: Serialize>(it: &mut I) -> Option<String>
where
    I: Iterator<Item = T>,
    //    I: std::fmt::Debug,
{
    let max = 1024 * 950;
    let mut lines = String::with_capacity(max);

    let mut size = 0;
    while size < max {
        match it.next() {
            None => {
                break;
            }
            Some(x) => {
                let json = match serde_json::to_string(&x) {
                    Ok(json) => json,
                    Err(err) => {
                        error!("Failed to serialize Item for Splunk:  {err}");
                        continue;
                    }
                };
                size += json.len();
                lines.push_str(json.as_str());
                lines.push('\n');
            }
        }
    }

    if lines.is_empty() {
        None
    } else {
        Some(lines)
    }
}

/// Run a future to completion and send the results to Splunk.
///
/// Logs the start / end / error messages of the collection to Splunk.
pub async fn try_collect_send<T>(
    name: &str,
    future: impl Future<Output = Result<T>>,
    splunk: &Splunk,
) -> Result<()>
where
    for<'a> &'a T: ToHecEvents + Debug,
{
    info!("Getting {}", &name);
    match future.await {
        Ok(ref result) => {
            let hec_events = match result.to_hec_events() {
                Ok(hec_events) => hec_events,
                Err(e) => {
                    warn!("Failed converting to HecEvents: {e}");
                    vec![HecEvent::new(
                        &Message {
                            event: format!("Failed converting to HecEvents: {e:?}"),
                        },
                        "data_ingester_rust",
                        "data_ingester_rust_logs",
                    )?]
                }
            };

            match splunk.send_batch(&hec_events).await {
                Ok(()) => info!("Sent {}", &name),
                Err(e) => {
                    warn!("Failed Sending to Splunk: {e}");
                }
            };
        }
        Err(err) => {
            warn!("Failed to get {name}: {err:?}")
        }
    };
    Ok(())
}
