use crate::splunk::{HecEvent, Splunk};
use crate::thread::SplunkTask;
use anyhow::{Context, Result};
use tracing_core::subscriber::Subscriber;
use tracing_core::Event;
use tracing_serde::AsSerde;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::Layer;
use tracing_subscriber::{EnvFilter, Registry};

pub fn start_splunk_tracing(splunk: Splunk, source: &str, sourcetype: &str) -> Result<()> {
    let stdout_log = tracing_subscriber::fmt::layer()
        .with_ansi(false)
        .compact()
        .with_writer(std::io::stderr);

    let splunk_filter: EnvFilter =
        EnvFilter::from_default_env().add_directive("data_ingester_splunk::thread=OFF".parse()?);

    let splunk_layer = SplunkLayer::new(splunk, source, sourcetype);

    //    let env_filter = EnvFilter::from_default_env();
    let subscriber = Registry::default()
        .with(stdout_log)
        .with(splunk_layer)
        .with(splunk_filter);

    tracing::subscriber::set_global_default(subscriber)
        .context("Setting global tracing subscriber to Splunk")
}

struct SplunkLayer {
    splunk_task: SplunkTask,
    source: String,
    sourcetype: String,
}

impl SplunkLayer {
    pub fn new(splunk: Splunk, source: &str, sourcetype: &str) -> Self {
        Self {
            splunk_task: SplunkTask::new(splunk),
            source: source.to_string(),
            sourcetype: sourcetype.to_string(),
        }
    }
}

/// Simple [tracing_subscriber::Layer] to send events to Splunk
impl<S: Subscriber> Layer<S> for SplunkLayer {
    fn on_event(&self, event: &Event<'_>, _ctx: tracing_subscriber::layer::Context<'_, S>) {
        let hec_event = HecEvent::new(
            &event.as_serde(),
            self.source.as_str(),
            self.sourcetype.as_str(),
        )
        .expect("Serialization should complete");
        _ = self.splunk_task.send(hec_event);
    }
}

#[cfg(test)]
mod test {
    use crate::splunk::Splunk;

    use super::SplunkLayer;
    use anyhow::{Context, Result};
    use data_ingester_supporting::keyvault::get_keyvault_secrets;
    use tracing::info;
    use tracing_subscriber::{prelude::*, Registry};

    #[tokio::test(flavor = "multi_thread")]
    async fn test_tracing() -> Result<()> {
        let secrets = get_keyvault_secrets(
            &std::env::var("KEY_VAULT_NAME").expect("Need KEY_VAULT_NAME enviornment variable"),
        )
        .await
        .unwrap();

        let splunk = Splunk::new(
            &secrets.splunk_host.as_ref().context("No value")?,
            &secrets.splunk_token.as_ref().context("No value")?,
        )?;

        let stdout_log = tracing_subscriber::fmt::layer().pretty();
        let subscriber = Registry::default().with(stdout_log);

        let subscriber = subscriber.with(SplunkLayer::new(
            splunk,
            "data_ingester_test",
            "data_ingester_test",
        ));
        let _tracing_guard = tracing::subscriber::set_default(subscriber);

        info!("This will be logged to stdout SPLUNK");
        info!("message");
        Ok(())
    }

    #[tracing::instrument]
    fn foo(n: usize) -> Result<()> {
        info!("inside foo");
        Ok(())
    }
}
