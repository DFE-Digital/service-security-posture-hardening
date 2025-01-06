use crate::splunk::{HecEvent, HecFields};
use anyhow::{Context, Result};
use itertools::Itertools;
use reqwest::{Client, Response, StatusCode};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::{
    sync::mpsc::{
        error::TryRecvError::{Disconnected, Empty},
        Receiver, Sender,
    },
    task::JoinHandle,
    time::{sleep, Duration, Instant},
};
use tracing::{error, info};

#[derive(Debug, Clone)]
pub(crate) struct HecBatch {
    ack_id: u32,
    batch: Vec<HecEvent>,
    sent_time: tokio::time::Instant,
}

/// Data recieved from Splunk after sending an event via HEC with
/// indexer acknowledgement
/// https://docs.splunk.com/Documentation/Splunk/9.3.2/Data/AboutHECIDXAck
#[derive(Debug, Serialize, Deserialize, Default)]
pub(crate) struct HecAckResponse {
    #[serde(rename = "ackId")]
    pub(crate) ack_id: u32,
    code: u32,
    text: String,
}

/// Query sent to Splunk to poll for Ack status
#[derive(Debug, Serialize, Deserialize, Default)]
pub(crate) struct HecAckQuery {
    pub(crate) acks: Vec<u32>,
}

/// The status of the polled for acks
#[derive(Debug, Serialize, Deserialize, Default)]
pub(crate) struct HecAckQueryResponse {
    pub(crate) acks: HashMap<String, bool>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub(crate) struct HecAckQueryResponseParsed {
    pub(crate) acks: HashMap<u32, bool>,
}

/// An async task to send messages to Splunk.
/// Listens on a channel for `HecEvent`s, serializes them, sends them
/// to Splunk HEC over HTTP, and forwards their details to the
/// `AckTask` for indexer acknowledgement
///
/// https://docs.splunk.com/Documentation/Splunk/9.3.2/Data/AboutHECIDXAck
///
pub(crate) struct SendingTask {
    #[allow(unused)]
    join: JoinHandle<Result<()>>,
}

impl SendingTask {
    pub(crate) fn new(
        splunk: Client,
        send_rx: Receiver<HecEvent>,
        ack_tx: Sender<HecBatch>,
        url: String,
        hec_acknowledgment: bool,
    ) -> Result<Self> {
        let url = format!("{}/services/collector", &url);
        let join = Self::spawn_task(splunk, send_rx, ack_tx, url, hec_acknowledgment)
            .context("Starting Splunk Sending Task")?;

        Ok(SendingTask { join })
    }

    // Send a batch to Splunk HEC
    async fn send_batch_to_splunk(
        splunk: &Client,
        batch: &str,
        url: &str,
    ) -> Result<HecAckResponse> {
        let mut retry_count = 0;
        let response: Response = loop {
            let response = splunk.post(url).body(batch.to_string()).send().await;

            let response = match response {
                Ok(response) => response,
                Err(err) => {
                    // Log error and resend batch if this fails.
                    error!(name="SplunkHec", operation="Send Hec payload", error=?err);
                    sleep(Duration::from_millis(200)).await;
                    if retry_count > 5 {
                        anyhow::bail!("Failed connecting to Splunk HEC");
                    } else {
                        retry_count += 1;
                        continue;
                    }
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

            // https://docs.splunk.com/Documentation/Splunk/8.2.0/Data/TroubleshootHTTPEventCollector
            // Status code	HTTP status code ID	HTTP status code	Status message
            // 0	        200	                OK               	Success
            // 1	        403             	Forbidden        	Token disabled
            // 2	        401               	Unauthorized	        Token is required
            // 3	        401	                Unauthorized	        Invalid authorization
            // 4	        403	                Forbidden       	Invalid token
            // 5	        400	                Bad Request       	No data
            // 6	        400              	Bad Request      	Invalid data format
            // 7	        400              	Bad Request	        Incorrect index
            // 8	        500             	Internal Error   	Internal server error
            // 9	        503	                Service Unavailable 	Server is busy
            // 10	        400	                Bad Request      	Data channel is missing
            // 11	        400              	Bad Request      	Invalid data channel
            // 12	        400              	Bad Request      	Event field is required
            // 13	        400               	Bad Request     	Event field cannot be blank
            // 14	        400	                Bad Request      	ACK is disabled
            // 15	        400	                Bad Request	        Error in handling indexed fields
            // 16	        400	                Bad Request     	Query string authorization is not enabled
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
            } else if status.as_u16() == 503 {
                log_failed(status);
                continue;
            } else if status.is_server_error() {
                log_failed(status);
                if retry_count > 5 {
                    anyhow::bail!("Server Error when sending HEC to Splunk");
                } else {
                    retry_count += 1;
                    continue;
                };
            }
        };

        let response_ack = match response.json::<HecAckResponse>().await {
            Ok(response_ack) => response_ack,
            Err(err) => {
                error!(name="SplunkHec", operation="Convert Hec response into HecAckResponse", error=?err);
                anyhow::bail!("Unable to convert HecAckResponse");
            }
        };

        Ok(response_ack)
    }

    async fn sending_task(
        splunk: Client,
        mut send_rx: Receiver<HecEvent>,
        ack_tx: Sender<HecBatch>,
        sending_url: String,
        hec_acknowledgment: bool,
    ) -> Result<()> {
        let mut buffer = Vec::with_capacity(100);
        loop {
            // Get messages from channel
            let received_count = send_rx.recv_many(&mut buffer, 100).await;

            // Break if channel is closed
            if received_count == 0 {
                break;
            }

            // Batch events for sending
            let batches: Vec<(String, Vec<HecEvent>)> =
                buffer.iter().batching(batch_events).collect();

            for batch in batches.into_iter() {
                // Send batch and receive the Ack code for the batch
                let response_ack =
                    Self::send_batch_to_splunk(&splunk, &batch.0, sending_url.as_str())
                        .await
                        .context("sending batch to Splunk")?;
                if hec_acknowledgment {
                    // Build a batch to enable resending
                    let hec_batch = HecBatch {
                        ack_id: response_ack.ack_id,
                        batch: batch.1,
                        sent_time: tokio::time::Instant::now(),
                    };

                    // Send batch to Ack Task
                    match ack_tx.reserve().await {
                        Ok(permit) => {
                            permit.send(hec_batch);
                        }
                        Err(err) => {
                            error!(operation="SplunkHec", operation="Reserve HecBatch on ack_task", error=?err)
                        }
                    }
                }
            }

            buffer.clear();
        }
        Ok(())
    }

    /// Spawn a new tokio task to send events to Splunk
    fn spawn_task(
        splunk: Client,
        send_rx: Receiver<HecEvent>,
        ack_tx: Sender<HecBatch>,
        url: String,
        hec_acknowledgment: bool,
    ) -> Result<JoinHandle<Result<()>>> {
        let join_handle = tokio::spawn(Self::sending_task(
            splunk,
            send_rx,
            ack_tx,
            url,
            hec_acknowledgment,
        ));
        Ok(join_handle)
    }
}

/// An async task to poll Splunk for HEC Ack statuses
///
/// If a message has failed to Ack after 5mins then send the 'failed'
/// events back to the `SendingTask` for retransmission.
pub(crate) struct AckTask {
    #[allow(unused)]
    join: JoinHandle<Result<()>>,
    #[allow(unused)]
    timeout: Duration,
}

impl AckTask {
    pub(crate) fn new(
        splunk: Client,
        send_tx: Sender<HecEvent>,
        ack_rx: Receiver<HecBatch>,
        url: String,
        timeout: Option<Duration>,
    ) -> Result<Self> {
        let url = format!("{}/services/collector/ack", &url);
        let timeout = if let Some(timeout) = timeout {
            timeout
        } else {
            Duration::from_secs(60 * 5)
        };
        let join = Self::spawn_task(splunk, ack_rx, send_tx, url, timeout)
            .context("Starting Splunk Ack Task")?;
        Ok(Self { join, timeout })
    }

    fn spawn_task(
        splunk: Client,
        ack_rx: Receiver<HecBatch>,
        send_tx: Sender<HecEvent>,
        url: String,
        timeout: Duration,
    ) -> Result<JoinHandle<Result<()>>> {
        let join_handle = tokio::spawn(Self::ack_task(splunk, ack_rx, send_tx, url, timeout));
        Ok(join_handle)
    }

    async fn send_hec_ack_query(
        splunk: &Client,
        ack_url: &str,
        to_be_acked: &HashMap<u32, HecBatch>,
    ) -> Result<HecAckQueryResponse> {
        let acks_for_query = to_be_acked.keys().copied().collect();
        let ack_query = HecAckQuery {
            acks: acks_for_query,
        };

        let response = splunk
            .post(ack_url.to_string())
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
        timeout: Duration,
    ) -> Result<()> {
        let mut to_be_acked: HashMap<u32, HecBatch> = HashMap::with_capacity(3200);
        let mut last_ack_time = Instant::now();
        let pause_between_acks = Duration::from_secs(1);
        'main: loop {
            // Get messages from channel

            'recv: loop {
                let ack = ack_rx.try_recv();
                match ack {
                    Ok(ack) => {
                        let _ = to_be_acked.insert(ack.ack_id, ack);
                    }
                    Err(err) => match err {
                        Empty => break 'recv,
                        Disconnected => break 'main,
                    },
                };
            }

            // If there is nothing to ACK, sleep and poll channel again
            if to_be_acked.is_empty() {
                sleep(Duration::from_millis(20)).await;
                continue;
            }

            // Only ack once per pause_between_acks interval
            if Instant::now() - last_ack_time < pause_between_acks {
                sleep(Duration::from_millis(20)).await;
                continue;
            }

            // Query SplunkHEC for ACK statuses
            let ack_response = Self::send_hec_ack_query(&splunk, &ack_url, &to_be_acked).await?;
            last_ack_time = Instant::now();

            // If the ack state is true, parse an int from the
            // returned string and remove it from future acks
            // and future batches.
            ack_response
                .acks
                .iter()
                .filter_map(|(_ack_id, state)| {
                    if *state {
                        match _ack_id.parse::<u32>() {
                            Ok(ack_id) => Some((ack_id, state)),
                            Err(err) => {
                                error!(name="SplunkHec", operation="HecAck", error=?err, "Unable to parse u32 from Splunk HecAckID");
                                None
                            },
                        }
                    } else {
                        None
                    }
                })
                .for_each(|(ack_id, state)| {
                    if *state && to_be_acked.remove(&ack_id).is_none() {
                        error!(name="Splunk", operation="HecAck", ack_id=?ack_id, "ack_id not found in known acks");
                    }
                });
            if !to_be_acked.is_empty() {
                let _ = Self::resend_events(&mut to_be_acked, send_tx.clone(), &timeout).await;
            }
        }
        Ok(())
    }

    async fn resend_events(
        to_be_acked: &mut HashMap<u32, HecBatch>,
        send_tx: Sender<HecEvent>,
        timeout: &Duration,
    ) -> Result<()> {
        let now = Instant::now();
        let pre_resend_to_be_acked_len = to_be_acked.len();

        let resend_batches: Vec<HecBatch> = to_be_acked
            .extract_if(|_k, hecbatch| {
                let live_time = now - hecbatch.sent_time;
                live_time > *timeout
            })
            .map(|(_k, batch)| batch)
            .collect();
        let post_resend_to_be_acked_len = to_be_acked.len();

        // Only send info! if there is a difference, used to reduce log spam
        if pre_resend_to_be_acked_len != post_resend_to_be_acked_len {
            info!(name="Splunk", operation="HecAck",
              pre_resend_to_be_acked_len=?pre_resend_to_be_acked_len,
              post_resend_to_be_acked_len=?post_resend_to_be_acked_len,
              "to_be_acked len()s");
        }

        for batch in resend_batches.into_iter() {
            for mut event in batch.batch.into_iter() {
                event.fields = if let Some(mut fields) = event.fields {
                    fields.resend_count += 1;
                    Some(fields)
                } else {
                    Some(HecFields { resend_count: 1 })
                };

                let send_result = send_tx.send(event).await;
                match send_result {
                    Ok(_) => {}
                    Err(err) => {
                        error!(name="SplunkHec", operation="Transmitting HecEvent to SendingTask", error=?err);
                    }
                };
            }
        }

        Ok(())
    }
}

/// Batch events for Splunk HEC
///
/// Split an iterator into batches of events based on their JSON serialized total size.
/// Each batch should be no larger than (1024 * 950) bytes
///
pub(crate) fn batch_events<'a, I>(it: &mut I) -> Option<(String, Vec<HecEvent>)>
where
    I: Iterator<Item = &'a HecEvent>,
{
    const MAX: usize = 1024 * 950;

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
                size += json.len();
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

#[cfg(test)]
mod test {
    use crate::{
        splunk::{HecEvent, Splunk},
        tasks::{AckTask, HecAckQueryResponse, HecAckResponse, HecBatch, SendingTask},
    };
    use mockito::{Matcher::Any, Server, ServerGuard};
    use std::collections::HashMap;
    use tokio::{
        sync::mpsc::{channel, Receiver, Sender},
        time::{sleep, Duration, Instant},
    };
    use tracing::subscriber::DefaultGuard;

    fn fake_event() -> HecEvent {
        let mut test_data = HashMap::new();
        let _ = test_data.insert("foo", "bar");
        HecEvent::new_with_ssphp_run(&test_data, "mocktest_source", "mocktest_sourcetype", 1)
            .expect("Building events should not fail")
    }

    async fn setup_send_task() -> (
        SendingTask,
        Sender<HecEvent>,
        Receiver<HecBatch>,
        ServerGuard,
        DefaultGuard,
    ) {
        // Start tracing
        let subscriber = tracing_subscriber::FmtSubscriber::new();
        let tracing_guard = tracing::subscriber::set_default(subscriber);

        // Start Mockito server
        let mock_server = Server::new_async().await;

        // Setup SendingTask
        let url = format!("http://{}", mock_server.host_with_port());
        let (send_tx, send_rx) = channel::<HecEvent>(1000);
        let (ack_tx, ack_rx) = channel(1000);
        let hec_acknowledgment = true;
        let client = Splunk::new_request_client("mock_token", hec_acknowledgment)
            .expect("Splunk Client to build sucessfully");
        let sending_task =
            SendingTask::new(client, send_rx, ack_tx.clone(), url, hec_acknowledgment)
                .expect("Spawning SendingTask shouldn't fail");

        (sending_task, send_tx, ack_rx, mock_server, tracing_guard)
    }

    fn mock_response() -> HecAckResponse {
        HecAckResponse {
            ack_id: 0,
            code: 200,
            text: "Success".into(),
        }
    }

    fn mock_response_body() -> String {
        serde_json::to_string(&mock_response()).expect("Serialization shouldn't fail")
    }

    async fn send_hec_event(send_tx: Sender<HecEvent>) {
        send_tx
            .send(fake_event())
            .await
            .expect("Sending on channel shouldn't fail");

        // Wait for SendingTask to make a HTTP request to Mockito
        sleep(Duration::from_millis(50)).await;
    }

    #[tokio::test]
    async fn test_sending_task_posts_to_correct_url() {
        let (_sending_task, send_tx, _ack_rx, mut mock_server, _tracing_guard) =
            setup_send_task().await;

        let mock = mock_server
            .mock("POST", "/services/collector")
            .with_status(200)
            .with_body(mock_response_body())
            .create();

        send_hec_event(send_tx).await;

        mock.assert();
    }

    #[tokio::test]
    async fn test_sending_task_posts_with_authorization_header() {
        let (_sending_task, send_tx, _ack_rx, mut mock_server, _tracing_guard) =
            setup_send_task().await;

        let mock = mock_server
            .mock("POST", "/services/collector")
            .match_header("authorization", "Splunk mock_token")
            .with_status(200)
            .with_body(mock_response_body())
            .create();

        send_hec_event(send_tx).await;

        mock.assert();
    }

    #[tokio::test]
    async fn test_sending_task_posts_with_x_splunk_request_header() {
        let (_sending_task, send_tx, _ack_rx, mut mock_server, _tracing_guard) =
            setup_send_task().await;

        let mock = mock_server
            .mock("POST", "/services/collector")
            .match_header("x-splunk-request-channel", Any)
            .with_status(200)
            .with_body(mock_response_body())
            .create();

        send_hec_event(send_tx).await;

        mock.assert();
    }

    #[tokio::test]
    async fn test_sending_task_posts_with_expected_body() {
        let (_sending_task, send_tx, _ack_rx, mut mock_server, _tracing_guard) =
            setup_send_task().await;

        let event = fake_event();
        let expected_request_body =
            serde_json::to_value(&event).expect("Serializing fake event shouldn't fail");

        let mock = mock_server
            .mock("POST", "/services/collector")
            .match_body(mockito::Matcher::AllOf(vec![mockito::Matcher::Json(
                expected_request_body,
            )]))
            .with_status(200)
            .with_body(mock_response_body())
            .create();

        let _ = send_tx.send(event).await;
        sleep(Duration::from_millis(50)).await;

        mock.assert();
    }

    #[tokio::test]
    async fn test_sending_task_creates_a_message_on_the_ack_channel() {
        let (_sending_task, send_tx, mut ack_rx, mut mock_server, _tracing_guard) =
            setup_send_task().await;

        let _mock = mock_server
            .mock("POST", "/services/collector")
            .with_status(200)
            .with_body(mock_response_body())
            .create();

        send_hec_event(send_tx).await;

        let ack_message = ack_rx.recv().await;
        assert!(ack_message.is_some());
    }

    #[tokio::test]
    async fn test_sending_task_ack_message_contains_ack_id() {
        let (_sending_task, send_tx, mut ack_rx, mut mock_server, _tracing_guard) =
            setup_send_task().await;

        let _mock = mock_server
            .mock("POST", "/services/collector")
            .with_status(200)
            .with_body(mock_response_body())
            .create();

        send_hec_event(send_tx).await;
        let mock_response = mock_response();
        let ack_message = ack_rx.recv().await.expect("To receive message");
        assert_eq!(ack_message.ack_id, mock_response.ack_id);
    }

    #[tokio::test]
    async fn test_sending_task_ack_message_contains_one_hec_event() {
        let (_sending_task, send_tx, mut ack_rx, mut mock_server, _tracing_guard) =
            setup_send_task().await;

        let _mock = mock_server
            .mock("POST", "/services/collector")
            .with_status(200)
            .with_body(mock_response_body())
            .create();

        send_hec_event(send_tx).await;

        let fake_event = fake_event();
        let ack_message = ack_rx.recv().await.expect("To receive message");
        assert_eq!(ack_message.batch.len(), 1);
        assert_eq!(ack_message.batch[0].source, fake_event.source);
        assert_eq!(ack_message.batch[0].sourcetype, fake_event.sourcetype);
    }

    async fn setup_ack_task(
        timeout: Option<Duration>,
    ) -> (
        AckTask,
        Sender<HecBatch>,
        Receiver<HecEvent>,
        ServerGuard,
        DefaultGuard,
    ) {
        // Start tracing
        let subscriber = tracing_subscriber::FmtSubscriber::new();
        let tracing_guard = tracing::subscriber::set_default(subscriber);

        // Setup Mockito
        let mock_server = Server::new_async().await;

        // Setup AckTask
        let url = format!("http://{}", mock_server.host_with_port());
        let (send_tx, send_rx) = channel::<HecEvent>(1000);
        let (ack_tx, ack_rx) = channel(1000);
        let client =
            Splunk::new_request_client("mock_token", true).expect("Splunk client to build");
        let ack_task = AckTask::new(client, send_tx, ack_rx, url, timeout)
            .expect("Spawning SendingTask shouldn't fail");

        (ack_task, ack_tx, send_rx, mock_server, tracing_guard)
    }
    fn ack_response_success() -> HecAckQueryResponse {
        let acks = [("0".to_string(), true)].into_iter().collect();
        HecAckQueryResponse { acks }
    }

    fn ack_response_body_success() -> String {
        serde_json::to_string(&ack_response_success()).expect("Serialization shouldn't fail")
    }

    fn ack_response_failure() -> HecAckQueryResponse {
        let acks = [("0".to_string(), false)].into_iter().collect();
        HecAckQueryResponse { acks }
    }

    fn ack_response_body_failure() -> String {
        serde_json::to_string(&ack_response_failure()).expect("Serialization shouldn't fail")
    }

    fn hec_batch() -> HecBatch {
        HecBatch {
            ack_id: 0,
            batch: vec![fake_event()],
            sent_time: Instant::now(),
        }
    }

    async fn send_hec_batch(ack_tx: Sender<HecBatch>) {
        ack_tx
            .send(hec_batch())
            .await
            .expect("Sending on channel shouldn't fail");

        // Wait for AckTask to make a HTTP request to Mockito
        sleep(Duration::from_millis(1500)).await;
    }

    #[tokio::test]
    async fn test_ack_task_posts_to_corrrect_url() {
        let (_ack_task, ack_tx, _send_rx, mut mock_server, _tracing_guard) =
            setup_ack_task(None).await;

        // Setup Mocks
        let mock = mock_server
            .mock("POST", "/services/collector/ack")
            .with_status(200)
            .with_body(ack_response_body_success())
            .create();

        send_hec_batch(ack_tx).await;

        mock.assert();
    }

    #[tokio::test]
    async fn test_ack_task_default_timeout() {
        let (ack_task, _ack_tx, _send_rx, _mock_server, _tracing_guard) =
            setup_ack_task(None).await;
        assert_eq!(ack_task.timeout, Duration::from_secs(60 * 5));
    }

    #[tokio::test]
    async fn test_ack_task_can_set_timeout() {
        let timeout = Duration::from_millis(1);
        let (ack_task, _ack_tx, _send_rx, _mock_server, _tracing_guard) =
            setup_ack_task(Some(timeout)).await;
        assert_eq!(ack_task.timeout, timeout);
    }

    #[tokio::test]
    async fn test_ack_task_does_not_retransmit_sucessully_indexed_events() {
        let (_ack_task, ack_tx, mut send_rx, mut mock_server, _tracing_guard) =
            setup_ack_task(Some(Duration::from_millis(100))).await;

        let _mock = mock_server
            .mock("POST", "/services/collector/ack")
            .with_status(200)
            .with_body(ack_response_body_success())
            .create();

        send_hec_batch(ack_tx).await;

        // Wait for AckTask to make a HTTP request to Mockito
        sleep(Duration::from_millis(200)).await;

        // Check we've haven't recieved a requset to retransmit
        let ack_message = send_rx.try_recv();
        assert!(ack_message.is_err());
    }

    #[tokio::test]
    async fn test_ack_task_does_retransmit_failed_events() {
        let (_ack_task, ack_tx, mut send_rx, mut mock_server, _tracing_guard) =
            setup_ack_task(Some(Duration::from_millis(1))).await;

        let _mock = mock_server
            .mock("POST", "/services/collector/ack")
            .with_status(200)
            .with_body(ack_response_body_failure())
            .create();

        send_hec_batch(ack_tx).await;

        // Wait for AckTask to make a HTTP request to Mockito
        sleep(Duration::from_millis(200)).await;

        let resend_event = send_rx.try_recv();
        assert!(resend_event.is_ok());
    }

    #[tokio::test]
    async fn test_ack_task_retransmit_increments_resend_count() {
        let (_ack_task, ack_tx, mut send_rx, mut mock_server, _tracing_guard) =
            setup_ack_task(Some(Duration::from_nanos(1))).await;

        let _mock = mock_server
            .mock("POST", "/services/collector/ack")
            .with_status(200)
            .with_body(ack_response_body_failure())
            .create();

        send_hec_batch(ack_tx).await;

        sleep(Duration::from_millis(200)).await;

        let resend_event = send_rx.try_recv().expect("Resend message should exist");

        assert_eq!(
            resend_event
                .fields
                .expect("fields should exist on retransmissions")
                .resend_count,
            1
        );
    }
}
