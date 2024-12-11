use anyhow::Context;
use itertools::Itertools;
use reqwest::header::HeaderMap;
use reqwest::header::HeaderValue;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::Sender;
use tokio::time::Instant;
// #[cfg(test)]
use std::borrow::Borrow;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::LazyLock;
use std::time::Duration;
use tracing::error;
use tracing::info;
use tracing::warn;
// #[cfg(test)]
use anyhow::{anyhow, Result};
use std::fmt::Debug;
use std::future::Future;
use std::sync::RwLock;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;
use tokio::sync::mpsc::channel;

use crate::tasks::AckTask;
use crate::tasks::HecAckQuery;
use crate::tasks::HecAckQueryResponse;
use crate::tasks::HecAckResponse;
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
pub(crate) struct HecFields {
    pub(crate) resend_count: i32,
}

impl HecEvent {
    pub fn new<T: Serialize>(event: &T, source: &str, sourcetype: &str) -> Result<HecEvent> {
        let ssphp_run = *SSPHP_RUN
            .read()
            .expect("Should always be able to read SSPHP_RUN");
        let ssphp_event = SsphpEvent {
            ssphp_run,
            event,
            resend_count: None,
        };
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
        let ssphp_event = SsphpEvent {
            ssphp_run,
            event,
            resend_count: None,
        };
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

    pub(crate) fn serialize(&self) -> Result<String> {
        Ok(serde_json::to_string(self)?)
    }
}
#[derive(Serialize, Deserialize)]
struct SsphpEvent<T> {
    #[serde(rename = "SSPHP_RUN")]
    ssphp_run: u64,
    #[serde(flatten)]
    event: T,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "SSPHP_RESEND_COUNT")]
    resend_count: Option<i32>,
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

//#[derive(Debug, Clone)]
pub struct Splunk {
//    pub(crate) client: Client,
//    pub(crate) url: String,
    pub(crate) sending_task: SendingTask,
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
    pub fn new(host: &str, token: &str) -> Result<Self> {
        let url = format!("https://{}/services/collector", host);
        let client = reqwest::ClientBuilder::new()
            .danger_accept_invalid_certs(true)
            .default_headers(Splunk::headers(token)?)
            .connection_verbose(false)
            .build()?;
        // let start = SystemTime::now();
        // let client_creation_time = start.duration_since(UNIX_EPOCH)?;
        let (send_tx, send_rx) = channel::<HecEvent>(1000);
        let (ack_tx, ack_rx) = channel(1000);

        let sending_task = SendingTask::new(client.clone(), send_rx, ack_tx.clone(), url.clone());
        let ack_task = AckTask::new(client.clone(), send_tx.clone(), ack_rx, url.clone());

        Ok(Self {
//            url,
            sending_task,
            ack_task,
            send_tx,
        })
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

    // #[cfg(test)]
    // pub async fn send(&self, event: &HecEvent) {
    //     let request = self.client.post(&self.url).json(event).build().unwrap();
    //     let _response = self.client.execute(request).await.unwrap();
    // }

    pub async fn send_batch(
        &self,
        events: impl IntoIterator<Item = HecEvent>,
    ) -> Result<()> {
        for event in events {
            self.send_tx.send(event).await?;
        }
        Ok(())
    }

    // /// Send a batch with indexer acknowledgement
    // pub async fn send_batch(
    //     &self,
    //     events: impl IntoIterator<Item = impl Borrow<HecEvent> + Serialize>,
    // ) -> Result<()> {
    //     let mut batches: Vec<String> = events.into_iter().batching(batch_lines).collect();

    //     let mut acks: HashMap<i32, usize> = HashMap::with_capacity(batches.len());

    //     let initial_batch_size = batches.len();

    //     let start_instant = Instant::now();
    //     let max_delay = Duration::from_secs(60);
    //     let mut resends = 0;

    //     let ack_url = format!("{}/ack", &self.url);

    //     #[allow(clippy::never_loop)]
    //     loop {
    //         // Send all batches
    //         for (index, batch) in batches.iter().enumerate() {
    //             let response_ack = self
    //                 .client
    //                 .post(&self.url)
    //                 .body(batch.clone())
    //                 .send()
    //                 .await
    //                 .context("sending to splunk")?
    //                 .json::<HecAckResponse>()
    //                 .await
    //                 .context("Convert Splunk HEC response to HecAckResponse")?;

    //             let _ = acks.insert(response_ack.ack_id, index);
    //         }

    //         //tokio::time::sleep(Duration::from_millis(0)).await;
    //         let mut delay_between_polls = Duration::from_millis(1);

    //         // Poll for acks util all complete/true or timeout is reached
    //         loop {
    //             // Check for Ack status
    //             let acks_for_query = acks.keys().copied().collect();
    //             let ack_query = HecAckQuery {
    //                 acks: acks_for_query,
    //             };

    //             let response = self
    //                 .client
    //                 .post(&ack_url)
    //                 .json(&ack_query)
    //                 .send()
    //                 .await
    //                 .context("Sending HecAckQuery to Splunk")?;

    //             let body = response.text().await.context("Getting ack query body")?;
    //             let ack_response = match serde_json::from_str::<HecAckQueryResponse>(&body) {
    //                 Ok(hec_ack_query_response) => hec_ack_query_response,
    //                 Err(err) => {
    //                     error!(name="SplunkHec", operation="HecAck", error=?err, ack_query=?ack_query, body=?body, "failed to build HecAckQueryResponse");
    //                     anyhow::bail!("failed to build HecAckQueryResponse");
    //                 }
    //             };

    //             // If the ack state is true, parse an int from the
    //             // returned string and remove it from future acks
    //             // and future batches.
    //             ack_response
    //                 .acks
    //                 .iter()
    //                 .filter_map(|(_ack_id, state)| {
    //                     if *state {
    //                         match _ack_id.parse::<i32>() {
    //                             Ok(ack_id) => Some(ack_id),
    //                             Err(err) => {
    //                                 error!(name="SplunkHec", operation="HecAck", error=?err, "Unable to parse i32 from Splunk HecAckID");
    //                                 None
    //                             },
    //                         }
    //                     } else {
    //                         None
    //                     }
    //                 })
    //                 .for_each(|ack_id| {
    //                     if let Some(batch_index) = acks.remove(&ack_id) {
    //                         let _ = batches.remove(batch_index);
    //                     } else {
    //                         error!(name="Splunk", operation="HecAck", "ack_id not found in known acks");
    //                     }
    //                 });

    //             // Break if we have successfully indexed all data
    //             if acks.is_empty() || batches.is_empty() {
    //                 break;
    //             }

    //             // Give up if we've waited too long
    //             if delay_between_polls > max_delay {
    //                 error!(
    //                     name = "SplunkHec",
    //                     operation = "HecAck",
    //                     unindexed_event_count = acks.len()
    //                 );
    //                 break;
    //             }

    //             // Wait between polls
    //             tokio::time::sleep(delay_between_polls).await;
    //             delay_between_polls *= 2;
    //         }

    //         // break if we've successfully indexed all data
    //         if acks.is_empty() || batches.is_empty() {
    //             break;
    //         } else {
    //             resends += 1;
    //         }

    //         // Do not resend events.
    //         // Remove this to enable retransmission of failed acks.
    //         break;
    //     }

    //     // Log events where we would have resent data
    //     if resends > 0 {
    //         let end_instant = Instant::now();
    //         let elapsed_time = (end_instant - start_instant).as_millis();
    //         info!(
    //             name = "SplunkHec",
    //             operation = "HecResend",
    //             elapsed_time = elapsed_time,
    //             initial_batch_size = initial_batch_size,
    //             resends = resends
    //         );
    //     }

    //     Ok(())
    // }
}

struct HecBatch {
    ack_id: i32,
    batch: String,
}

// Needs Splunk Creds

pub(crate) fn batch_lines<I, T: Serialize>(it: &mut I) -> Option<String>
where
    I: Iterator<Item = T>,
    //    I: std::fmt::Debug,
{
    const MAX: usize = 1024 * 950;
    const resend_size_increase: usize = ",\"SSPHP_RESEND_COUNT\":1".len();

    let mut lines = String::with_capacity(MAX);

    let mut size: usize = 0;
    while size < MAX {
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
                size += json.len() + resend_size_increase;
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

pub(crate) fn batch_lines_events<'a, I>(it: &mut I) -> Option<(String, Vec<HecEvent>)>
where
    I: Iterator<Item = &'a HecEvent>,
    //    T: Serialize,
    //    I: std::fmt::Debug,
{
    const MAX: usize = 1024 * 950;
    const resend_size_increase: usize = ",\"SSPHP_RESEND_COUNT\":1".len();

    let mut lines = String::with_capacity(MAX);
    let mut events = Vec::new();
    let mut size: usize = 0;
    while size < MAX {
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
                size += json.len() + resend_size_increase;
                lines.push_str(json.as_str());
                lines.push('\n');
                events.push(x.clone());
            }
        }
    }

    if lines.is_empty() {
        None
    } else {
        Some((lines, events))
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
        )?;
        Ok(splunk)
    }

    #[tokio::test]
    async fn send_to_splunk() -> Result<()> {
        let splunk = splunk_client().await?;

        let data = std::collections::HashMap::from([("aktest", "fromrust")]);
        let he = HecEvent::new(&data, "msgraph_rust", "test_event").unwrap();
        splunk.send_batch([he]).await;
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
