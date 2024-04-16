use crate::splunk::{HecEvent, Splunk};
use anyhow::Result;
use tokio::sync::mpsc::unbounded_channel;
use tokio::task::JoinHandle;
use tracing::instrument;

pub struct SplunkTask {
    tx: Option<tokio::sync::mpsc::UnboundedSender<HecEvent>>,
    join_handle: Option<JoinHandle<()>>,
}

impl SplunkTask {
    /// Start a collector listening for events to send to Splunk    
    pub fn new(splunk: Splunk) -> SplunkTask {
        let (tx, rx) = unbounded_channel::<HecEvent>();
        let join_handle = tokio::spawn(async move { Self::process_events(splunk, rx).await });
        SplunkTask {
            tx: Some(tx),
            join_handle: Some(join_handle),
        }
    }

    pub fn send(&self, event: HecEvent) -> Result<()> {
        if let Some(tx) = self.tx.as_ref() {
            tx.send(event)?;
        } else {
            anyhow::bail!("Channel closed");
        }
        Ok(())
    }

    #[instrument]
    async fn process_events(
        splunk: Splunk,
        mut rx: tokio::sync::mpsc::UnboundedReceiver<HecEvent>,
    ) {
        const AVG_EVENT_SIZE: usize = 341;
        const LIMIT_SPLUNK_HEC_BYTES: usize = 1_000_000;
        const LIMIT_EVENTS: usize = LIMIT_SPLUNK_HEC_BYTES / AVG_EVENT_SIZE;
        dbg!(LIMIT_EVENTS);
        let mut events = Vec::with_capacity(LIMIT_EVENTS);
        loop {
            let count = rx.recv_many(&mut events, LIMIT_EVENTS).await;

            // Is the channel closed?
            if count == 0 {
                break;
            }

            match splunk.send_batch(&events).await {
                Ok(_) => {
                    events.clear();
                    continue;
                }
                Err(err) => eprintln!("Failed to send event to Splunk: {}", err),
            }
        }
    }
}

/// Custom Drop Impl to make sure channel is closed and remaining events are sent to Splunk.
impl Drop for SplunkTask {
    fn drop(&mut self) {
        // Drop the channel
        drop(self.tx.take());
        // Get the join handle for the logging Future
        let jh = self.join_handle.take().expect("join handle should exist");
        // Get a [tokio::runtime::Handle] to the current tokio runtime
        let handle = tokio::runtime::Handle::current();
        // Spawn a thead to run the logging to completion blocking the current thread until it completes.
        // This makes sure events are sent before dropping
        std::thread::spawn(move || handle.block_on(jh))
            .join()
            .expect("SplunkTask to complete successfully")
            .expect("No errors produced by SplunkTask");
    }
}

#[cfg(test)]
mod test {
    use crate::{
        splunk::{HecEvent, Splunk},
        thread::SplunkTask,
    };
    use anyhow::{Context, Result};
    use data_ingester_supporting::keyvault::get_keyvault_secrets;

    #[tokio::test(flavor = "multi_thread")]
    async fn test_background_task() -> Result<()> {
        let secrets = get_keyvault_secrets(
            &std::env::var("KEY_VAULT_NAME").expect("Need KEY_VAULT_NAME enviornment variable"),
        )
        .await
        .unwrap();

        let splunk = Splunk::new(&secrets.splunk_host, &secrets.splunk_token)
            .context("building Splunk client")?;

        let splunk_task = SplunkTask::new(splunk);

        let data = serde_json::from_str::<serde_json::Value>(r#"{"data": "test"}"#)?;
        let event = HecEvent::new(&data, "threadtest", "threadtest")?;
        splunk_task.send(event)?;
        drop(splunk_task);
        Ok(())
    }
}
