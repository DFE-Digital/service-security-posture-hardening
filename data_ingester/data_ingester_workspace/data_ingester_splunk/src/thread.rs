use std::sync::Arc;

use crate::splunk::{HecEvent, Splunk};
use anyhow::Result;
use tokio::sync::mpsc::unbounded_channel;
use tokio::task::JoinHandle;
use tracing::error;

pub struct SplunkTask {
    tx: Option<tokio::sync::mpsc::UnboundedSender<HecEvent>>,
    join_handle: Option<JoinHandle<()>>,
}

impl SplunkTask {
    /// Start a collector listening for events to send to Splunk    
    pub fn new(splunk: Arc<Splunk>) -> SplunkTask {
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

    async fn process_events(
        splunk: Arc<Splunk>,
        mut rx: tokio::sync::mpsc::UnboundedReceiver<HecEvent>,
    ) {
        loop {
            let event = match rx.recv().await {
                Some(event) => event,
                // Is the channel closed?
                None => break,
            };

            match splunk.send_batch([event]).await {
                Ok(_) => {
                    continue;
                }
                Err(err) => error!("Failed to send event to Splunk: {}", err),
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

#[cfg(feature = "live_tests")]
#[cfg(test)]
mod live_tests {
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

        let splunk = Splunk::new(
            secrets.splunk_host.as_ref().context("No value")?,
            secrets.splunk_token.as_ref().context("No value")?,
        )?;

        let splunk_task = SplunkTask::new(splunk.into());

        let data = serde_json::from_str::<serde_json::Value>(r#"{"data": "test"}"#)?;
        let event = HecEvent::new(&data, "threadtest", "threadtest")?;
        splunk_task.send(event)?;
        drop(splunk_task);
        Ok(())
    }
}
