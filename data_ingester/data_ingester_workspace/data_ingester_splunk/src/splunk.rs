use anyhow::{anyhow, Context, Result};
use itertools::Itertools;
use reqwest::{
    header::{HeaderMap, HeaderValue},
    Client, ClientBuilder,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fmt::Debug,
    future::Future,
    sync::{LazyLock, RwLock},
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::sync::mpsc::{channel, Sender};
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::tasks::AckTask;
use crate::tasks::SendingTask;

// Legacy, just used for tests and logs
static SSPHP_RUN: RwLock<u64> = RwLock::new(0_u64);

pub(crate) static SSPHP_RUN_NEW: LazyLock<RwLock<HashMap<String, u64>>> = LazyLock::new(|| {
    let mut hm = HashMap::new();
    let _ = hm.insert("default".to_string(), 0_u64);
    RwLock::new(hm)
});

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HecEvent {
    pub source: String,
    pub sourcetype: String,
    pub host: String,
    pub event: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fields: Option<HecFields>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HecFields {
    pub resend_count: i32,
}

impl HecEvent {
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
            fields: None,
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
            fields: None,
        })
    }

    pub fn increase_resend_count(&mut self) {
        if let Some(ref mut fields) = self.fields {
            fields.resend_count += 1;
        } else {
            self.fields = Some(HecFields { resend_count: 1 })
        };
    }
}

#[cfg(test)]
mod test_hec_event {
    use std::collections::HashMap;

    use super::HecEvent;

    #[test]
    fn increase_resend_count() {
        let mut event =
            HecEvent::new::<HashMap<String, String>>(&HashMap::new(), "source", "sourcetype")
                .expect("To be able to build HecEvent");
        assert!(event.fields.is_none());
        event.increase_resend_count();
        assert!(event.fields.is_some());
        assert_eq!(
            event
                .fields
                .as_ref()
                .map(|fields| fields.resend_count)
                .unwrap(),
            1
        );
        event.increase_resend_count();
        assert_eq!(
            event
                .fields
                .as_ref()
                .map(|fields| fields.resend_count)
                .unwrap(),
            2
        );
    }
}
#[derive(Serialize, Deserialize)]
struct SsphpEvent<T: Serialize> {
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
                .unwrap_or_else(|| hm.get("default").unwrap_or(&0))
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
                    .unwrap_or_else(|| hm.get("default").unwrap_or(&0))
            })
            .unwrap_or_else(|_| 0);
        ssphp_run
    }
    fn ssphp_run_key(&self) -> &str;
}

pub struct Splunk {
    #[allow(unused)]
    pub(crate) sending_task: SendingTask,
    #[allow(unused)]
    pub(crate) ack_task: AckTask,
    pub(crate) send_tx: Sender<HecEvent>,
}

unsafe impl Send for Splunk {}
unsafe impl Sync for Splunk {}

#[derive(Debug, Serialize, Deserialize, Default)]
pub(crate) struct Message {
    pub event: String,
}

impl Splunk {
    pub fn new(host: &str, token: &str, hec_acknowledgment: bool) -> Result<Self> {
        let url = format!("https://{}", host);

        let client = Self::new_request_client(token, hec_acknowledgment)
            .context("Building Reqwest Client")?;

        let (send_tx, send_rx) = channel::<HecEvent>(5000);
        let (ack_tx, ack_rx) = channel(5000);

        let sending_task = SendingTask::new(
            client.clone(),
            send_rx,
            ack_tx.clone(),
            url.clone(),
            hec_acknowledgment,
        )
        .context("Building Splunk Sending Task")?;

        let ack_task = AckTask::new(client.clone(), send_tx.clone(), ack_rx, url.clone(), None)
            .context("Building Splunk Ack Task")?;

        Ok(Self {
            sending_task,
            ack_task,
            send_tx,
        })
    }

    /// Create a Request Client for Splunk
    pub(crate) fn new_request_client(token: &str, hec_acknowledgment: bool) -> Result<Client> {
        let accept_invalid_certs = std::env::var_os("ACCEPT_INVALID_CERTS").is_some();

        let client = ClientBuilder::new()
            .danger_accept_invalid_certs(accept_invalid_certs)
            .default_headers(Splunk::headers(token, hec_acknowledgment)?)
            .connection_verbose(false)
            .build()?;
        Ok(client)
    }

    fn headers(token: &str, hec_acknowledgment: bool) -> Result<HeaderMap> {
        let mut headers = HeaderMap::new();
        let mut auth = HeaderValue::from_str(&format!("Splunk {}", token))?;
        auth.set_sensitive(true);
        _ = headers.insert("Authorization", auth);

        if hec_acknowledgment {
            let channel = Uuid::new_v4().to_string();
            _ = headers.insert("X-Splunk-Request-Channel", channel.parse()?);
        }

        Ok(headers)
    }

    pub async fn send_batch(&self, events: impl IntoIterator<Item = HecEvent>) -> Result<()> {
        for event in events {
            match self.send_tx.reserve().await {
                Ok(permit) => {
                    permit.send(event);
                }
                Err(err) => {
                    error!(operation="SplunkHec", operation="Reserve HecBatch on self.send_tx failed", error=?err)
                }
            }
        }
        Ok(())
    }
}

/// Run a future to completion and send the results to Splunk.
///
/// Logs the start / end / error messages of the collection to Splunk.
/// TODO Fix this so logging is not excluded.
pub async fn try_collect_send<T>(
    name: &str,
    future: impl Future<Output = Result<T>>,
    splunk: &Splunk,
) -> Result<T>
where
    for<'a> &'a T: ToHecEvents + Debug,
{
    info!("Getting {}", &name);
    let result = future.await;
    match &result {
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

            match splunk.send_batch(hec_events).await {
                Ok(()) => {
                    info!("Sent {}", &name);
                }
                Err(e) => {
                    warn!("Failed Sending to Splunk: {e}");
                }
            };
        }
        Err(err) => {
            warn!("Failed to get {name}: {err:?}")
        }
    };
    result
}
#[cfg(test)]
pub(crate) mod test {
    use crate::splunk::Splunk;
    #[tokio::test]
    async fn splunk_headers_with_hec_ack_should_set_request_channel_header() {
        let hec_acknowledgment = true;
        let headers = Splunk::headers("token", hec_acknowledgment).unwrap();
        assert!(headers.contains_key("X-Splunk-Request-Channel"));
    }

    #[tokio::test]
    async fn splunk_headers_without_hec_ack_should_not_set_request_channel_header() {
        let hec_acknowledgment = false;
        let headers = Splunk::headers("token", hec_acknowledgment).unwrap();
        assert!(!headers.contains_key("X-Splunk-Request-Channel"));
    }
}

#[cfg(feature = "live_tests")]
#[cfg(test)]
pub(crate) mod live_tests {
    use crate::splunk::{HecEvent, Splunk};
    use anyhow::{Context, Result};
    use data_ingester_supporting::keyvault::get_keyvault_secrets;
    use std::{collections::HashMap, env};

    pub(crate) async fn splunk_client() -> Result<Splunk> {
        let secrets = get_keyvault_secrets(&env::var("KEY_VAULT_NAME")?).await?;
        let splunk = Splunk::new(
            secrets.splunk_host.as_ref().context("No value")?,
            secrets.splunk_token.as_ref().context("No value")?,
            true,
        )?;
        Ok(splunk)
    }

    #[tokio::test]
    async fn send_to_splunk() -> Result<()> {
        let splunk = splunk_client().await?;

        let data = std::collections::HashMap::from([("aktest", "fromrust")]);
        let he = HecEvent::new(&data, "msgraph_rust", "test_event").unwrap();
        splunk.send_batch([he]).await?;
        Ok(())
    }

    #[tokio::test]
    async fn send_batch_to_splunk() -> Result<()> {
        let splunk = splunk_client().await?;

        let mut events = Vec::new();
        let data = HashMap::from([("aktest0", "fromrust")]);
        let he = HecEvent::new(&data, "msgraph_rust", "test_event").unwrap();
        events.push(he);

        let data1 = HashMap::from([("aktest1", "fromrust")]);
        let he1 = HecEvent::new(&data1, "msgraph_rust", "test_event").unwrap();
        events.push(he1);
        splunk.send_batch(events).await.unwrap();
        Ok(())
    }
}
