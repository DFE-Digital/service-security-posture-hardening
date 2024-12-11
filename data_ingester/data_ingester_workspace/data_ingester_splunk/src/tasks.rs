use crate::splunk::HecEvent;
use crate::splunk::HecFields;
use crate::splunk::Splunk;
use anyhow::Context;
use anyhow::Result;
use itertools::Itertools;
use reqwest::Client;
use reqwest::Response;
use reqwest::StatusCode;
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc::channel;
use tokio::sync::mpsc::Receiver;
use tokio::sync::mpsc::Sender;
use tokio::task::JoinHandle;
use tokio::time::Instant;
use tracing::error;

#[derive(Debug)]
pub(crate) struct HecBatch {
    ack_id: i32,
    batch: Vec<HecEvent>,
    sent_time: tokio::time::Instant,
    // events: Vec<HecEvent>
}

/// Data recieved from Splunk after sending an event via HEC with
/// indexer acknowledgement
/// https://docs.splunk.com/Documentation/Splunk/9.3.2/Data/AboutHECIDXAck
#[derive(Debug, Serialize, Deserialize, Default)]
pub(crate) struct HecAckResponse {
    #[serde(rename = "ackId")]
    pub(crate) ack_id: i32,
    code: i32,
    text: String,
}

/// Query sent to Splunk to poll for Ack status
#[derive(Debug, Serialize, Deserialize, Default)]
pub(crate) struct HecAckQuery {
    pub(crate) acks: Vec<i32>,
}

/// The status of the polled for acks
#[derive(Debug, Serialize, Deserialize, Default)]
pub(crate) struct HecAckQueryResponse {
    pub(crate) acks: HashMap<String, bool>,
}

struct Tasks {
    sending_task: SendingTask,
    ack_task: AckTask,
}

//#[derive(Debug, Clone)]
pub(crate) struct AckTask {
    splunk: Client,
    join: Option<JoinHandle<Result<()>>>,
    send_tx: Option<Sender<HecEvent>>,
    //    send_tx: Option<Sender<HecBatch>>,
    ack_rx: Option<Receiver<HecBatch>>,
    //    ack_tx: Option<Sender<HecBatch>>,
    url: String,
}

impl AckTask {
    pub(crate) fn new(
        splunk: Client,
        send_tx: Sender<HecEvent>,
        ack_rx: Receiver<HecBatch>,
        url: String,
    ) -> Self {
        Self {
            splunk,
            join: None,
            send_tx: Some(send_tx),
            ack_rx: Some(ack_rx),
            url,
        }
    }

    fn spawn_task(&mut self) -> Result<()> {
        if self.join.is_some() {
            anyhow::bail!("This sender has already spawned");
        }

        if self.ack_rx.is_none() {
            anyhow::bail!("This sender has already spawned");
        }

        let ack_url = format!("{}/ack", &self.url);
        let splunk = self.splunk.clone();
        let ack_rx = self
            .ack_rx
            .take()
            .expect("Sending Channel must be initialised");
        let send_tx = self.send_tx.clone().expect("ack_tx should be present");

        let join_handle = tokio::spawn(Self::ack_task(splunk, ack_rx, send_tx, ack_url));
        self.join = Some(join_handle);
        Ok(())
    }

    async fn send_hec_ack_query(
        splunk: &Client,
        ack_url: &str,
        to_be_acked: &HashMap<i32, HecBatch>,
    ) -> Result<HecAckQueryResponse> {
        let acks_for_query = to_be_acked.keys().copied().collect();
        let ack_query = HecAckQuery {
            acks: acks_for_query,
        };

        let response = splunk
            .post(&ack_url.to_string())
            .json(&ack_query)
            .send()
            .await
            .context("Sending HecAckQuery to Splunk")?;

        let body = response.text().await.context("Getting ack query body")?;

        let ack_response = match serde_json::from_str::<HecAckQueryResponse>(&body) {
            Ok(hec_ack_query_response) => hec_ack_query_response,
            Err(err) => {
                error!(name="SplunkHec", operation="HecAck", error=?err, ack_query=?ack_query, body=?body, "failed to build HecAckQueryResponse");
                anyhow::bail!("failed to build HecAckQueryResponse");
            }
        };

        Ok(ack_response)
    }

    async fn ack_task(
        splunk: Client,
        mut ack_rx: Receiver<HecBatch>,
        send_tx: Sender<HecEvent>,
        ack_url: String,
    ) -> Result<()> {
        let mut to_be_acked: HashMap<i32, HecBatch> = HashMap::with_capacity(3200);

        'main: loop {
            // Get messages from channel
            loop {
                let ack = ack_rx.try_recv();
                match ack {
                    Ok(ack) => {
                        dbg!(&ack);
                        let _ = to_be_acked.insert(ack.ack_id, ack);
                    }
                    Err(err) => match err {
                        tokio::sync::mpsc::error::TryRecvError::Empty => break,
                        tokio::sync::mpsc::error::TryRecvError::Disconnected => break 'main,
                    },
                };
            }

            dbg!(&to_be_acked);
            if to_be_acked.is_empty() {
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                continue;
            }
            let ack_response = Self::send_hec_ack_query(&splunk, &ack_url, &to_be_acked).await?;
            dbg!(&ack_response);

            // If the ack state is true, parse an int from the
            // returned string and remove it from future acks
            // and future batches.
            ack_response
                .acks
                .iter()
                .filter_map(|(_ack_id, state)| {
                    if *state {
                        match _ack_id.parse::<i32>() {
                            Ok(ack_id) => Some((ack_id, state)),
                            Err(err) => {
                                error!(name="SplunkHec", operation="HecAck", error=?err, "Unable to parse i32 from Splunk HecAckID");
                                None
                            },
                        }
                    } else {
                        None
                    }
                })
                .for_each(|(ack_id, state)| {
                    dbg!(&ack_id, state);
                    if *state {
                        if to_be_acked.remove(&ack_id).is_none() {
                            error!(name="Splunk", operation="HecAck", ack_id=?ack_id, "ack_id not found in known acks");
                        }
                    }
                });
            let _ = Self::resend_events(&mut to_be_acked, send_tx.clone()).await;
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        }
        Ok(())
    }

    async fn resend_events(
        to_be_acked: &mut HashMap<i32, HecBatch>,
        send_tx: Sender<HecEvent>,
    ) -> Result<()> {
        let now = Instant::now();

        const TIMEOUT: Duration = Duration::from_millis(15);

        let resend_batches: Vec<HecBatch> = to_be_acked
            .extract_if(|_k, hecbatch| {
                let live_time = now - hecbatch.sent_time;
                live_time > TIMEOUT
            })
            .map(|(_k, batch)| batch)
            .collect();

        for batch in resend_batches.into_iter() {
            for mut event in batch.batch.into_iter() {
                event.fields = if let Some(mut fields) = event.fields {
                    fields.resend_count += 1;
                    Some(fields)
                } else {
                    Some(HecFields { resend_count: 1 })
                };

                send_tx.send(event).await;
            }
        }
        Ok(())
    }
}

pub(crate) struct SendingTask {
    splunk: Client,
    join: Option<JoinHandle<Result<()>>>,
    send_rx: Option<Receiver<HecEvent>>,
    //    send_tx: Option<Sender<String>>,
    // send_rx: Option<Receiver<HecEvent>>,
    // send_tx: Option<Sender<HecEvent>>,
    ack_tx: Option<Sender<HecBatch>>,
    url: String,
}

impl SendingTask {
    pub(crate) fn new(
        splunk: Client,
        send_rx: Receiver<HecEvent>,
        ack_tx: Sender<HecBatch>,
        url: String,
    ) -> Self {
        SendingTask {
            splunk,
            join: None,
            send_rx: Some(send_rx),
            ack_tx: Some(ack_tx),
            url
        }
    }

    async fn send_batch_to_splunk(splunk: &Client, batch: &str, url: &str) -> Result<HecAckResponse> {
        let response: Response = loop {
            let response = splunk
                .post(url.clone())
                .body(batch.to_string())
                .send()
                .await;

            let response = match response {
                Ok(response) => response,
                Err(err) => {
                    // Log error and resend batch if this fails.
                    error!(name="SplunkHec", operation="Send Hec payload", error=?err);
                    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
                    continue;
                }
            };

            let status = response.status();

            let log_failed = |status: StatusCode| {
                let code = status.as_u16();
                let reason = status.canonical_reason();
                error!(
                    name = "SplunkHec",
                    operation = "Send Hec payload, non 200 response",
                    code = code,
                    reason = reason
                );
            };

            if status.is_success() {
                break response;
            } else if status.is_client_error() {
                log_failed(status);
                anyhow::bail!("Client error response when sending to Splunk. Should not happen");
            } else if status.is_informational() {
                log_failed(status);
                anyhow::bail!("informational response when sending to Splunk. Should not happen");
            } else if status.is_redirection() {
                log_failed(status);
                anyhow::bail!("redirect when sending to Splunk. Should not happen");
            } else if status.is_server_error() {
                log_failed(status);
                continue;
            }
        };

        let response_ack = response
            .json::<HecAckResponse>()
            .await
            .context("Unable to deserialize Splunk Response as HecAckResponse")?;
        Ok(response_ack)
    }

    async fn sending_task(
        splunk: Client,
        mut send_rx: Receiver<HecEvent>,
        ack_tx: Sender<HecBatch>,
        sending_url: String,
    ) -> Result<()> {
        let mut buffer = Vec::with_capacity(100);
        loop {
            // Get messages from channel
            let received_count = send_rx.recv_many(&mut buffer, 100).await;
            dbg!(&received_count);
            dbg!(&buffer);

            // Break if channel is closed
            if received_count == 0 {
                break;
            }

            let batches: Vec<(String, Vec<HecEvent>)> = buffer
                .iter()
                .batching(crate::splunk::batch_lines_events)
                .collect();

            for batch in batches.into_iter() {
                dbg!(&batch);
                let response_ack = Self::send_batch_to_splunk(&splunk, &batch.0, sending_url.as_str())
                    .await
                    .context("sending batch to Splunk")?;

                let hec_batch = HecBatch {
                    ack_id: response_ack.ack_id,
                    batch: batch.1,
                    sent_time: tokio::time::Instant::now(),
                };

                match ack_tx.send(hec_batch).await {
                    Ok(_) => {}
                    Err(err) => {
                        error!(operation="SplunkHec", operation="Send Ack & Batch to ack_task", error=?err);
                    }
                }
            }

            buffer.clear();
        }
        Ok(())
    }

    fn spawn_task(&mut self) -> Result<()> {
        if self.join.is_some() {
            anyhow::bail!("This sender has already spawned");
        }

        if self.send_rx.is_none() {
            anyhow::bail!("send_rx is not set");
        }

        let join_handle = tokio::spawn(Self::sending_task(
            self.splunk.clone(),
            self.send_rx
                .take()
                .expect("Sending Channel must be initialised"),
            self.ack_tx.take().expect("ack_tx should be set"),
            self.url.clone(),
        ));
        self.join = Some(join_handle);
        Ok(())
    }
}

#[cfg(feature = "live_tests")]
#[cfg(test)]
mod test {
    use std::{collections::HashMap, sync::Arc};

    use crate::{
        splunk::{live_tests::splunk_client, HecEvent},
        tasks::{AckTask, SendingTask},
    };
    use anyhow::Result;
    use tokio::sync::mpsc::channel;

    // #[tokio::test]
    // async fn build_sending_task() -> Result<()> {
    //     let splunk = splunk_client().await?;
    //     let splunk = Arc::new(splunk);
    //     let (send_tx, send_rx) = channel::<HecEvent>(1000);
    //     let (ack_tx, ack_rx) = channel(1000);
    //     let mut sending_task = SendingTask::new(splunk.clone(), send_rx, ack_tx.clone(), splunk.url.clone());
    //     let mut ack_task = AckTask::new(splunk, send_tx.clone(), ack_rx);
    //     sending_task.spawn_task()?;
    //     ack_task.spawn_task()?;
    //     let mut test_data = HashMap::new();
    //     for i in 0..1000 {
    //         test_data.insert("foo", "bar");
    //         let test_hec_event = HecEvent::new_with_ssphp_run(&test_data, "test", "test", i)?;
    //         send_tx.send(test_hec_event).await;
    //         tokio::time::sleep(tokio::time::Duration::from_millis(20)).await;
    //     }

    //     //let text = test_hec_event.serialize()?;

    //     tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
    //     assert!(false);
    //     Ok(())
    // }
}
