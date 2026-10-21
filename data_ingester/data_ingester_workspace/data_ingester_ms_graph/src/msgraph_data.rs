use std::collections::HashMap;
use std::sync::Arc;

use crate::ms_graph::MsGraph;
use anyhow::Result;
use data_ingester_splunk::splunk::{to_hec_events, Splunk, SplunkTrait};
use serde::Deserialize;
use serde_json::Value;
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
                // A feed naming a property in `explode` is split into one event per element of
                // that property. Every other feed has `None` here and sends `result` unchanged.
                let exploded = ms_graph_source.explode().map(|property| {
                    result
                        .iter()
                        .flat_map(|value| explode_value(value, property))
                        .collect::<Vec<Value>>()
                });
                let to_send: &[Value] = exploded.as_deref().unwrap_or(result);
                let hec_events = match to_hec_events(
                    to_send,
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
                match splunk.send_batch(hec_events).await {
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
    /// Splits each returned record into one event per element of this property.
    ///
    /// Set it only where a single record is too large for Splunk to extract - see
    /// `explode_value` for why that happens and what it costs. Left unset, which is every
    /// other feed, the records are sent exactly as Graph returned them.
    explode: Option<String>,
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
    fn explode(&self) -> Option<&str> {
        self.explode.as_deref()
    }
}

/// Splits one returned record into one event per element of an array property.
///
/// WHY THIS EXISTS. Splunk stops extracting JSON fields past `[kv] maxchars`, 10,240 by default,
/// and an oversized record arrives whole and yields NO fields at all rather than some of them -
/// measured on the user feed at 23.8% of records. A servicePrincipal's `appRoles` is a single array
/// of several hundred role definitions and is far past that limit, which is why `appRoles` has been
/// excluded from the `service_principals` `$select` ever since that feed was written. Emitting one
/// event per role collects the same data in records Splunk can actually read.
///
/// WHAT EACH EVENT CARRIES. The element itself, plus the parent's own scalar properties so the
/// event names the application it came from without a join back - for `graph_app_roles` that is
/// `id`, `appId` and `displayName`. The parent's other arrays and nested objects are deliberately
/// NOT copied: they are what made the record oversized in the first place. An element that is not
/// an object is wrapped under the property name so it is still a well-formed event.
///
/// WHAT IT WILL NOT DO IS LOSE A RECORD. A record whose named property is absent, null, not an
/// array, or an empty array is passed through unchanged rather than dropped, so a feed that opts in
/// cannot go quietly empty if Graph stops returning the property or renames it.
fn explode_value(parent: &Value, property: &str) -> Vec<Value> {
    let Some(items) = parent.get(property).and_then(Value::as_array) else {
        return vec![parent.clone()];
    };
    if items.is_empty() {
        return vec![parent.clone()];
    }

    let parent_scalars: Vec<(String, Value)> = parent
        .as_object()
        .into_iter()
        .flatten()
        .filter(|(key, value)| key.as_str() != property && !value.is_array() && !value.is_object())
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect();

    items
        .iter()
        .map(|item| {
            let mut event = match item {
                Value::Object(map) => map.clone(),
                other => {
                    let mut map = serde_json::Map::new();
                    _ = map.insert(property.to_owned(), other.clone());
                    map
                }
            };
            for (key, value) in &parent_scalars {
                _ = event.entry(key.clone()).or_insert(value.clone());
            }
            Value::Object(event)
        })
        .collect()
}

#[cfg(test)]
mod test {

    use crate::msgraph_data::{explode_value, load_m365_toml};
    use anyhow::Result;
    use serde_json::json;

    #[test]
    fn test_toml_load() -> Result<()> {
        let sources = load_m365_toml()?;
        assert!(!sources.is_empty());
        Ok(())
    }

    /// `get_url` joins the endpoint onto the client's base URL, so a path with a
    /// leading slash replaces the base path and any endpoint without `/v1.0` or
    /// `/beta` loses its API version. Graph then reads the first path segment as
    /// the version and returns an error document, which is indexed as data.
    #[test]
    fn test_endpoints_carry_an_api_version() -> Result<()> {
        let sources = load_m365_toml()?;
        for group in sources.0.values() {
            for (name, source) in group {
                assert!(
                    source.endpoint.starts_with("/v1.0/") || source.endpoint.starts_with("/beta/"),
                    "{name}: endpoint has no API version segment: {}",
                    source.endpoint
                );
            }
        }
        Ok(())
    }

    /// One event per element, each carrying the parent's scalars so it stands on its own.
    #[test]
    fn explode_splits_an_array_property_into_one_event_each() {
        let parent = json!({
            "id": "sp-object-id",
            "appId": "00000003-0000-0000-c000-000000000000",
            "displayName": "Microsoft Graph",
            "appRoles": [
                { "id": "role-1", "value": "Directory.ReadWrite.All" },
                { "id": "role-2", "value": "User.Read.All" },
            ],
        });

        let events = explode_value(&parent, "appRoles");

        assert_eq!(events.len(), 2);
        assert_eq!(events[0]["value"], json!("Directory.ReadWrite.All"));
        assert_eq!(events[1]["value"], json!("User.Read.All"));
        for event in &events {
            assert_eq!(
                event["appId"],
                json!("00000003-0000-0000-c000-000000000000")
            );
            assert_eq!(event["displayName"], json!("Microsoft Graph"));
            assert!(
                event.get("appRoles").is_none(),
                "the exploded property must not be copied onto the event"
            );
        }
    }

    /// An element's own property wins over the parent's, so exploding cannot rewrite the data.
    #[test]
    fn explode_does_not_let_a_parent_scalar_overwrite_an_element() {
        let parent = json!({
            "id": "parent-id",
            "appRoles": [{ "id": "role-id", "value": "Directory.Read.All" }],
        });

        let events = explode_value(&parent, "appRoles");

        assert_eq!(events.len(), 1);
        assert_eq!(events[0]["id"], json!("role-id"));
    }

    /// A record is never dropped: absent, null, not an array and empty all pass through whole.
    #[test]
    fn explode_passes_a_record_through_rather_than_losing_it() {
        for parent in [
            json!({ "id": "no-such-property" }),
            json!({ "id": "null-property", "appRoles": null }),
            json!({ "id": "not-an-array", "appRoles": "Directory.Read.All" }),
            json!({ "id": "empty-array", "appRoles": [] }),
        ] {
            let events = explode_value(&parent, "appRoles");
            assert_eq!(events.len(), 1, "{parent} should pass through as one event");
            assert_eq!(events[0], parent, "{parent} should pass through unchanged");
        }
    }

    /// An element that is not an object is still a well-formed event.
    #[test]
    fn explode_wraps_a_scalar_element_under_the_property_name() {
        let parent = json!({ "id": "parent-id", "tags": ["one", "two"] });

        let events = explode_value(&parent, "tags");

        assert_eq!(events.len(), 2);
        assert_eq!(events[0]["tags"], json!("one"));
        assert_eq!(events[0]["id"], json!("parent-id"));
    }

    /// Exploding CHANGES THE SHAPE of a feed, so retrofitting it onto a feed something already
    /// searches would break that search. This pins the opt-in list: adding a feed here is a
    /// deliberate act, and the only record that needs it is the Graph app role definitions, whose
    /// single oversized record Splunk cannot extract at all.
    #[test]
    fn only_the_graph_app_roles_feed_explodes() -> Result<()> {
        let sources = load_m365_toml()?;
        let mut exploding: Vec<&str> = sources
            .0
            .values()
            .flatten()
            .filter(|(_, source)| source.explode.is_some())
            .map(|(name, _)| name.as_str())
            .collect();
        exploding.sort_unstable();
        assert_eq!(exploding, vec!["graph_app_roles"]);
        Ok(())
    }
}

#[cfg(feature = "live_tests")]
#[cfg(test)]
mod live_tests {
    use std::{env, sync::Arc};

    use crate::{ms_graph::MsGraph, msgraph_data::load_m365_toml};
    use anyhow::{Context, Result};
    use data_ingester_splunk::splunk::Splunk;
    use data_ingester_splunk::splunk::SplunkTrait;
    use data_ingester_supporting::keyvault::get_keyvault_secrets;

    #[tokio::test]
    async fn test_toml_live() -> Result<()> {
        let sources = load_m365_toml()?;

        let secrets = get_keyvault_secrets(&env::var("KEY_VAULT_NAME")?).await?;
        let ms_graph = MsGraph::new(
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
            true,
        )?);

        sources
            .process_sources(&ms_graph, &splunk, "default")
            .await?;

        Ok(())
    }
}
