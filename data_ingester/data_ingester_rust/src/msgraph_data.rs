use std::collections::HashMap;
use std::sync::Arc;

use crate::ms_graph::MsGraph;
use crate::splunk::{to_hec_events, HecEvent, Message, Splunk};
use anyhow::Result;
use serde::Deserialize;

/// Loads data from the ms_graph.toml file
pub fn load_m365_toml() -> Result<MsGraphData> {
    let contents = include_str!("../ms_graph.toml");

    let decoded: MsGraphData = toml::from_str(contents).unwrap();
    println!("{:#?}", decoded);
    Ok(decoded)
}

#[derive(Deserialize, Debug)]
pub struct MsGraphData(HashMap<String, HashMap<String, MsGraphSource>>);

impl MsGraphData {
    pub async fn process_sources(&self, ms_graph: &MsGraph, splunk: &Arc<Splunk>) -> Result<()> {
        for ms_graph_sources in self.0.values() {
            // dbg!(&section, &ms_graph_sources);
            for (source_name, ms_graph_source) in ms_graph_sources {
                // dbg!(&source_name, &ms_graph_source);

                MsGraphData::try_collect_send(source_name, ms_graph_source, ms_graph, splunk)
                    .await?;
            }
        }
        Ok(())
    }

    async fn try_collect_send(
        source_name: &str,
        ms_graph_source: &MsGraphSource,
        ms_graph: &MsGraph,
        splunk: &Splunk,
    ) -> Result<()> {
        let log_name = &format!("{}: {:?}", source_name, ms_graph_source);
        splunk.log(&format!("Getting {}", &log_name)).await?;
        match ms_graph.get_url(&ms_graph_source.endpoint).await {
            Ok(ref result) => {
                let hec_events = match to_hec_events(
                    result,
                    ms_graph_source.source(),
                    ms_graph_source.sourcetype(),
                ) {
                    Ok(hec_events) => hec_events,
                    Err(e) => {
                        eprintln!("Failed converting to HecEvents: {}", e);
                        dbg!(&result);
                        vec![HecEvent::new(
                            &Message {
                                event: format!("Failed converting to HecEvents: {}", e),
                            },
                            "data_ingester_rust",
                            "data_ingester_rust_logs",
                        )?]
                    }
                };

                match splunk.send_batch(&hec_events).await {
                    Ok(_) => eprintln!("Sent to Splunk"),
                    Err(e) => {
                        eprintln!("Failed Sending to Splunk: {}", e);
                        dbg!(&hec_events);
                    }
                };
            }
            Err(err) => {
                splunk
                    .log(&format!("Failed to get {}: {}", &log_name, err))
                    .await?
            }
        };
        Ok(())
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
pub struct MsGraphSource {
    endpoint: String,
    source: Option<String>,
    sourcetype: Option<String>,
    #[allow(dead_code)]
    cis_controls: Option<Vec<CisControl>>,
    #[allow(dead_code)]
    permissions_required: Option<Vec<String>>,
}

#[derive(Deserialize, Debug)]
struct CisControl {
    #[allow(dead_code)]
    filename: Option<String>,
    #[allow(dead_code)]
    control_id: Option<String>,
    #[allow(dead_code)]
    control_logic: Option<String>,
}

impl MsGraphSource {
    fn source(&self) -> &str {
        match self.source.as_ref() {
            Some(source) => source.as_str(),
            None => self.endpoint.as_str(),
        }
    }
    fn sourcetype(&self) -> &str {
        match self.sourcetype.as_ref() {
            Some(sourcetype) => sourcetype.as_str(),
            None => "ssphp:ms_graph:json",
        }
    }
}

#[cfg(test)]
mod test {
    use std::{env, sync::Arc};

    use crate::{
        keyvault::get_keyvault_secrets, ms_graph::login, msgraph_data::load_m365_toml,
        splunk::Splunk,
    };
    use anyhow::Result;

    #[test]
    fn test_toml() -> Result<()> {
        let sources = load_m365_toml()?;
        assert!(sources.len() > 0);
        Ok(())
    }

    #[tokio::test]
    async fn test_toml1() -> Result<()> {
        let sources = load_m365_toml()?;

        let secrets = get_keyvault_secrets(&env::var("KEY_VAULT_NAME")?).await?;
        let ms_graph = login(
            &secrets.azure_client_id,
            &secrets.azure_client_secret,
            &secrets.azure_tenant_id,
        )
        .await?;

        let splunk = Arc::new(Splunk::new(&secrets.splunk_host, &secrets.splunk_token)?);

        sources.process_sources(&ms_graph, &splunk).await?;

        Ok(())
    }
}
