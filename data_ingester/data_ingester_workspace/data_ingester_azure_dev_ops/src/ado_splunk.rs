use crate::ado_metadata::AdoMetadata;
use crate::ado_metadata::AdoMetadataTrait;
use anyhow::Result;
use data_ingester_splunk::splunk::Splunk;
use data_ingester_splunk::splunk::SplunkTrait;
use data_ingester_splunk::splunk::ToHecEvents;
use itertools::Itertools;
use serde::Serialize;
use tracing::warn;

pub(crate) struct AdoToSplunk();

impl AdoToSplunk {
    pub(crate) fn from_metadata(metadata: &AdoMetadata) -> AdoToSplunkBuilder {
        AdoToSplunkBuilder { metadata }
    }
}

pub(crate) struct AdoToSplunkBuilder<'metadata> {
    metadata: &'metadata AdoMetadata,
}

impl<'metadata> AdoToSplunkBuilder<'metadata> {
    /// Add a slice of events which will be sent as single events to Splunk
    #[allow(dead_code)]
    pub(crate) fn events<'t, T: Serialize>(
        self,
        events: &'t [T],
    ) -> AdoToHecEvents<'metadata, 't, T> {
        AdoToHecEvents {
            inner: events,
            metadata: self.metadata,
        }
    }

    /// Send a single event to Splunk.
    /// T cannot be a collection.
    pub(crate) fn event<'t, T: Serialize>(self, event: &'t T) -> AdoToHecEvent<'metadata, 't, T> {
        AdoToHecEvent {
            inner: event,
            metadata: self.metadata,
        }
    }
}

pub(crate) struct AdoToHecEvent<'metadata, 't, T: Serialize> {
    inner: &'t T,
    metadata: &'metadata AdoMetadata,
}

impl<'metadata, 't, T: Serialize> AdoToHecEvent<'metadata, 't, T> {
    pub(crate) async fn send(self, splunk: &Splunk) -> Result<()> {
        let events = self.to_hec_events()?;
        splunk.send_batch(events).await
    }
}

impl<'metadata, 't, T: Serialize> ToHecEvents for AdoToHecEvent<'metadata, 't, T> {
    type Item = T;

    fn source(&self) -> &str {
        self.metadata.metadata_source()
    }

    fn sourcetype(&self) -> &str {
        self.metadata.metadata_sourcetype()
    }

    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        Box::new(std::iter::once(self.inner))
    }

    fn ssphp_run_key(&self) -> &str {
        crate::SSPHP_RUN_KEY
    }

    fn to_hec_events(&self) -> Result<Vec<data_ingester_splunk::splunk::HecEvent>> {
        let mut event = serde_json::to_value(self.inner)?;
        let metadata = serde_json::to_value(self.metadata).unwrap_or_else(|_| {
            serde_json::to_value("Error Getting AdoMetadata")
                .expect("Value from static str should not fail")
        });
        let _ = event
            .as_object_mut()
            .expect("ado_response should always be accessible as an Value object")
            .insert("metadata".into(), metadata);
        let event = data_ingester_splunk::splunk::HecEvent::new_with_ssphp_run(
            &event,
            self.source(),
            self.sourcetype(),
            self.get_ssphp_run(),
        )?;
        Ok(vec![event])
    }
}

pub(crate) struct AdoToHecEvents<'metadata, 't, T: Serialize> {
    pub(crate) inner: &'t [T],
    pub(crate) metadata: &'metadata AdoMetadata,
}

impl<'metadata, 't, T: Serialize> AdoToHecEvents<'metadata, 't, T> {
    pub(crate) async fn send(self, splunk: &Splunk) -> Result<()> {
        let events = self.to_hec_events()?;
        splunk.send_batch(events).await
    }
}

impl<'metadata, 't, T: Serialize> ToHecEvents for AdoToHecEvents<'metadata, 't, T> {
    type Item = T;

    fn source(&self) -> &str {
        self.metadata.metadata_source()
    }

    fn sourcetype(&self) -> &str {
        self.metadata.metadata_sourcetype()
    }

    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        Box::new(self.inner.iter())
    }

    fn ssphp_run_key(&self) -> &str {
        crate::SSPHP_RUN_KEY
    }

    fn to_hec_events(&self) -> Result<Vec<data_ingester_splunk::splunk::HecEvent>> {
        let (ok, err): (Vec<_>, Vec<_>) = self
            .collection()
            .map(|event| {
                let mut event = serde_json::to_value(event)?;
                let metadata = serde_json::to_value(self.metadata).unwrap_or_else(|_| {
                    serde_json::to_value("Error Getting AdoMetadata")
                        .expect("Value from static str should not fail")
                });
                if let Some(object)  = event
                    .as_object_mut(){
                        let _ = object.insert("metadata".into(), metadata);
                    } else {
                        warn!(name=crate::SSPHP_RUN_KEY, event=?event, metadata=?metadata, "Failed to add `metadata` to event: unable to address event as_object_mut()");
                    };
                data_ingester_splunk::splunk::HecEvent::new_with_ssphp_run(
                    &event,
                    self.source(),
                    self.sourcetype(),
                    self.get_ssphp_run(),
                )
            })
            .partition_result();
        if !err.is_empty() {
            return Err(anyhow::anyhow!(err
                .iter()
                .map(|err| format!("{:?}", err))
                .collect::<Vec<String>>()
                .join("\n")));
        }
        Ok(ok)
    }
}
