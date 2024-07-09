use std::collections::HashMap;
use std::sync::Arc;

use crate::ms_graph::MsGraph;
use anyhow::Result;
use data_ingester_splunk::splunk::{to_hec_events, Splunk};
use serde::Deserialize;
use tracing::{error, info};

/// Loads data from the ms_graph.toml file
pub fn load_m365_toml() -> Result<MsGraphData> {
    let contents = include_str!("../ms_graph.toml");

    let decoded: MsGraphData =
        toml::from_str(contents).expect("ms_graph.toml should exist and be valid");
    Ok(decoded)
}

#[derive(Deserialize, Debug)]
pub struct MsGraphData(HashMap<String, HashMap<String, MsGraphSource>>);

impl MsGraphData {
    pub async fn process_sources(
        &self,
        ms_graph: &MsGraph,
        splunk: &Arc<Splunk>,
        ssphp_run_key: &str,
    ) -> Result<()> {
        for ms_graph_sources in self.0.values() {
            for (source_name, ms_graph_source) in ms_graph_sources {
                MsGraphData::try_collect_send(
                    source_name,
                    ms_graph_source,
                    ms_graph,
                    splunk,
                    ssphp_run_key,
                )
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
        ssphp_run_key: &str,
    ) -> Result<()> {
        let log_name = &format!("{}: {:?}", source_name, ms_graph_source);
        info!("Getting {}", &log_name);
        match ms_graph.get_url(&ms_graph_source.endpoint).await {
            Ok(ref result) => {
                let hec_events = match to_hec_events(
                    result,
                    ms_graph_source.source(),
                    ms_graph_source.sourcetype(),
                    ssphp_run_key,
                ) {
                    Ok(hec_events) => hec_events,
                    Err(e) => {
                        error!("Failed converting to HecEvents: {}", e);
                        vec![]
                    }
                };

                match splunk.send_batch(&hec_events).await {
                    Ok(_) => info!("Sent to Splunk"),
                    Err(e) => {
                        error!("Failed Sending to Splunk: {}", e);
                    }
                };
            }
            Err(err) => {
                error!("Failed to get {}: {}", &log_name, err);
            }
        };
        Ok(())
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
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

    use crate::msgraph_data::load_m365_toml;
    use anyhow::Result;

    #[test]
    fn test_toml_load() -> Result<()> {
        let sources = load_m365_toml()?;
        assert!(!sources.is_empty());
        Ok(())
    }
}

#[cfg(feature = "live_tests")]
#[cfg(test)]
mod live_tests {
    use std::{env, sync::Arc};

    use crate::{ms_graph::login, msgraph_data::load_m365_toml};
    use anyhow::{Context, Result};
    use data_ingester_splunk::splunk::Splunk;
    use data_ingester_supporting::keyvault::get_keyvault_secrets;

    #[tokio::test]
    async fn test_toml_live() -> Result<()> {
        let sources = load_m365_toml()?;

        let secrets = get_keyvault_secrets(&env::var("KEY_VAULT_NAME")?).await?;
        let ms_graph = login(
            secrets
                .azure_client_id
                .as_ref()
                .context("Expect azure_client_id secret")?,
            secrets
                .azure_client_secret
                .as_ref()
                .context("Expect azure_client_secret secret")?,
            secrets
                .azure_tenant_id
                .as_ref()
                .context("Expect azure_tenant_id secret")?,
        )
        .await?;

        let splunk = Arc::new(Splunk::new(
            secrets.splunk_host.as_ref().context("No value")?,
            secrets.splunk_token.as_ref().context("No value")?,
        )?);

        sources
            .process_sources(&ms_graph, &splunk, "default")
            .await?;

        Ok(())
    }
}
