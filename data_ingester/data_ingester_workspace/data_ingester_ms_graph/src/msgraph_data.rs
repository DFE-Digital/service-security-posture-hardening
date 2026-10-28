use std::collections::HashMap;
use std::sync::Arc;

use crate::ms_graph::MsGraph;
use anyhow::Result;
use data_ingester_splunk::splunk::{to_hec_events, Splunk, SplunkTrait};
use futures::StreamExt;
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
                // A feed declaring `refetch_at_cap` has any expanded collection that arrived at
                // Graph's cap replaced with the complete list from the relationship's own
                // endpoint. A feed that does not declare it, and a record below the cap, cost
                // nothing here.
                let completed = match (ms_graph_source.explode(), ms_graph_source.refetch_at_cap())
                {
                    (Some(property), Some(url_template)) => Some(
                        refetch_capped_collections(
                            result,
                            property,
                            url_template,
                            |url| async move { ms_graph.get_url(&url).await },
                        )
                        .await,
                    ),
                    _ => None,
                };
                let records: &[Value] = completed.as_deref().unwrap_or(result);

                // A feed naming a property in `explode` is split into one event per element of
                // that property. Every other feed has `None` here and sends `records` unchanged.
                let exploded = ms_graph_source.explode().map(|property| {
                    records
                        .iter()
                        .flat_map(|value| explode_value(value, property))
                        .collect::<Vec<Value>>()
                });
                let to_send: &[Value] = exploded.as_deref().unwrap_or(records);
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
    /// Set it where a single record is too large for Splunk to extract, or where it could
    /// become so if Graph stops honouring a query parameter it does not guarantee - see
    /// `explode_value` for why that happens and what it costs. Left unset, which is every
    /// other feed, the records are sent exactly as Graph returned them.
    explode: Option<String>,
    /// Replaces an expanded collection that arrived at Graph's cap with the complete list
    /// from the relationship's own endpoint, which is not capped. `{id}` is substituted with
    /// the record's `id`.
    ///
    /// Requires `explode` to name the property. See `refetch_capped_collections` for why the
    /// cap cannot simply be paged.
    refetch_at_cap: Option<String>,
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
    fn refetch_at_cap(&self) -> Option<&str> {
        self.refetch_at_cap.as_deref()
    }
}

/// Graph's documented cap on an expanded collection of directory objects.
///
/// Microsoft's known-issues list is explicit about every half of the problem: `$expand` on a
/// relationship of a directory object "Returns a maximum of 20 objects", there is "No support
/// for `@odata.nextLink`", and `$count` cannot be combined with it. So the overflow cannot be
/// paged and NOTHING IN THE RESPONSE SAYS THE LIST WAS CUT - the count reaching the cap is the
/// only signal there is. The one exception is `/users?$expand=registeredDevices`, which returns
/// up to 100 objects; no feed here uses it.
const EXPANSION_CAP: usize = 20;

/// Prefix under which `explode_value` keeps a parent scalar whose name the element also uses.
const PARENT_PREFIX: &str = "parent_";

/// The URL that would complete a record's capped collection, or `None` if it is not capped.
///
/// Split out from the request so the trigger can be tested without a tenant. A list at or over
/// the cap is assumed incomplete because the cap is the only signal Graph gives; a list below it
/// is complete and must not cost a request.
fn capped_refetch_url(record: &Value, property: &str, url_template: &str) -> Option<String> {
    let at_cap = record
        .get(property)
        .and_then(Value::as_array)
        .is_some_and(|items| items.len() >= EXPANSION_CAP);
    if !at_cap {
        return None;
    }
    let id = record.get("id").and_then(Value::as_str)?;
    Some(url_template.replace("{id}", id))
}

/// Replaces every capped collection in `records` with the complete list from the relationship's
/// own endpoint.
///
/// WHY THIS EXISTS. `$expand` is one request for the whole tenant and is what keeps these feeds
/// cheap, but it truncates at `EXPANSION_CAP` and says nothing about having done so. For
/// ownership that is not cosmetic: an owner of an application can add credentials to it and an
/// owner of a group can add themselves to it, so a list cut at twenty reads as less privilege
/// than exists. A relationship's own endpoint - `/v1.0/groups/{id}/owners` - is an ordinary
/// paged collection with no cap, so it is the complete answer; it just costs a request per
/// object. This is the same shape as `get_transitive_members_of_for_users`, which expands at the
/// collection level AND re-reads per principal for exactly this reason.
///
/// WHAT IT COSTS. Nothing until a collection actually reaches the cap. The sweep stays one
/// request per feed and only the records at the cap are re-read, at a concurrency of 10 to match
/// the per-principal paths. Measured 2026-10-28 the most owners on any object in this tenant was
/// 12, mean 2.5, with none at the cap, so today this makes no requests at all.
///
/// A FAILED REFETCH IS MARKED, NOT HIDDEN. The record keeps the capped list and gains
/// `{property}_incomplete = true`, which `explode_value` carries onto every event it produces.
/// A short list that nothing flags is the failure this function exists to prevent, and a refetch
/// that failed leaves precisely that - so it has to say so rather than pass for complete. One
/// object that cannot be read never abandons the feed.
/// `fetch` is injected rather than taken as a `&MsGraph` so that both branches below - the
/// replacement and the failure marker - can be tested without a tenant. They are the branches
/// that decide whether ownership data is complete, and they are reached only when something
/// already has twenty owners, so they would otherwise first run in production on the day they
/// matter.
async fn refetch_capped_collections<F, Fut>(
    records: &[Value],
    property: &str,
    url_template: &str,
    fetch: F,
) -> Vec<Value>
where
    F: Fn(String) -> Fut,
    Fut: std::future::Future<Output = Result<Vec<Value>>>,
{
    let tasks: Vec<_> = records
        .iter()
        .enumerate()
        .filter_map(|(index, record)| {
            let url = capped_refetch_url(record, property, url_template)?;
            let fetch = &fetch;
            Some(async move {
                info!(
                    url = url.as_str(),
                    "Expanded collection arrived at Graph's cap, re-reading it in full"
                );
                let result = fetch(url).await;
                (index, result)
            })
        })
        .collect();

    let mut out = records.to_vec();
    if tasks.is_empty() {
        return out;
    }

    let mut stream = futures::stream::iter(tasks).buffer_unordered(10);
    while let Some((index, result)) = stream.next().await {
        let Some(record) = out[index].as_object_mut() else {
            continue;
        };
        match result {
            Ok(complete) => {
                info!(
                    property = property,
                    count = complete.len(),
                    "Completed a capped collection"
                );
                _ = record.insert(property.to_owned(), Value::Array(complete));
            }
            Err(error) => {
                error!(
                    error = ?error,
                    property = property,
                    "Failed to complete a capped collection, marking the record incomplete"
                );
                _ = record.insert(format!("{property}_incomplete"), Value::Bool(true));
            }
        }
    }

    out
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
/// A COLLIDING PARENT SCALAR IS KEPT UNDER `parent_`, NOT DROPPED. The element owns its own key -
/// an exploded `appRole`'s `id` is the role's id, which is what a search for it expects - but the
/// parent's value is the only thing naming what the element belongs to, so losing it loses the
/// relationship. Both an owner and the object it owns carry `id` and `displayName`, so without
/// this an exploded owner event says who the owner is and nothing whatever about what they own,
/// which is the entire point of the feed.
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
                let key = if event.contains_key(key) {
                    format!("{PARENT_PREFIX}{key}")
                } else {
                    key.clone()
                };
                _ = event.entry(key).or_insert(value.clone());
            }
            Value::Object(event)
        })
        .collect()
}

#[cfg(test)]
mod test {

    use crate::msgraph_data::{
        capped_refetch_url, explode_value, load_m365_toml, refetch_capped_collections,
        EXPANSION_CAP,
    };
    use anyhow::{anyhow, Result};
    use serde_json::json;
    use serde_json::Value;

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
        assert_eq!(
            events[0]["parent_id"],
            json!("parent-id"),
            "a colliding parent scalar must be kept under parent_, not dropped: without it an \
             exploded owner names the owner and nothing about the object they own"
        );
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

    /// The owners feeds MUST keep their nested `$select`. Without it Graph returns a full
    /// user object per owner - about 2,850 characters each - and a group with twelve owners
    /// is one 34,817 character record. Splunk stops extracting past `[kv] maxchars`, 10,240,
    /// and an oversized record yields NO fields at all, so 27 of 100 groups sampled on
    /// 2026-10-28 would have arrived unreadable. With the nested `$select` the worst case is
    /// 2,725. This pins that, because the endpoint still looks correct without it.
    #[test]
    fn the_owners_feeds_select_inside_the_expand() -> Result<()> {
        let sources = load_m365_toml()?;
        let mut checked = 0;
        for group in sources.0.values() {
            for (name, source) in group {
                if !name.ends_with("_owners") {
                    continue;
                }
                checked += 1;
                assert!(
                    source.endpoint.contains("$expand=owners($select="),
                    "{name}: owners must be expanded with a nested $select or the records \
                     exceed Splunk maxchars: {}",
                    source.endpoint
                );
            }
        }
        assert_eq!(checked, 3, "expected three owners feeds");
        Ok(())
    }

    /// Exploding CHANGES THE SHAPE of a feed, so retrofitting it onto a feed something already
    /// searches would break that search. This pins the opt-in list: adding a feed here is a
    /// deliberate act. Graph's app role definitions need it because the single record cannot be
    /// extracted at all; the three owners feeds need it so that a record can never exceed
    /// maxchars even if Graph stops honouring the nested `$select` it does not guarantee.
    #[test]
    fn only_the_intended_feeds_explode() -> Result<()> {
        let sources = load_m365_toml()?;
        let mut exploding: Vec<&str> = sources
            .0
            .values()
            .flatten()
            .filter(|(_, source)| source.explode.is_some())
            .map(|(name, _)| name.as_str())
            .collect();
        exploding.sort_unstable();
        assert_eq!(
            exploding,
            vec![
                "application_owners",
                "graph_app_roles",
                "group_owners",
                "service_principal_owners",
            ]
        );
        Ok(())
    }

    /// THE SHAPE THE DETECTIONS WILL READ, pinned on a record in the form Graph actually
    /// returns one. An owner and the object it owns both carry `id` and `displayName`, so the
    /// owner takes those keys and the owned object keeps its own under `parent_`. A search for
    /// "who owns this group" reads `parent_id`; a search for "what does this person own" reads
    /// `id` and `userPrincipalName`. Getting this wrong silently produces a feed that names
    /// owners and not what they own, which is indistinguishable from working.
    #[test]
    fn an_exploded_owner_names_both_the_owner_and_the_object_owned() {
        let group = json!({
            "id": "group-object-id",
            "displayName": "Infrastructure Admins",
            "owners": [
                { "id": "owner-1", "userPrincipalName": "a@example.gov.uk", "displayName": "A" },
                { "id": "owner-2", "userPrincipalName": "b@example.gov.uk", "displayName": "B" },
            ],
        });

        let events = explode_value(&group, "owners");

        assert_eq!(events.len(), 2, "one event per owner");
        assert_eq!(events[0]["id"], json!("owner-1"), "the owner takes `id`");
        assert_eq!(events[0]["userPrincipalName"], json!("a@example.gov.uk"));
        assert_eq!(events[0]["displayName"], json!("A"));
        assert_eq!(
            events[0]["parent_id"],
            json!("group-object-id"),
            "the object owned must still be identifiable without a join back"
        );
        assert_eq!(
            events[0]["parent_displayName"],
            json!("Infrastructure Admins")
        );
        assert_eq!(events[1]["id"], json!("owner-2"));
        assert_eq!(events[1]["parent_id"], json!("group-object-id"));
        assert!(
            events[0].get("owners").is_none(),
            "the exploded property must not be carried as well"
        );
    }

    /// A refetch that failed leaves a list cut at the cap, and the marker is the only thing
    /// separating that from a complete list of twenty. It has to reach every event, because the
    /// events are what gets searched - the record they came from is not.
    #[test]
    fn the_incomplete_marker_reaches_every_exploded_event() {
        let group = json!({
            "id": "group-object-id",
            "owners_incomplete": true,
            "owners": [
                { "id": "owner-1" },
                { "id": "owner-2" },
            ],
        });

        let events = explode_value(&group, "owners");

        assert_eq!(events.len(), 2);
        for event in &events {
            assert_eq!(
                event["owners_incomplete"],
                json!(true),
                "every event from a record whose refetch failed must say so"
            );
        }
    }

    /// `$expand` on a directory relationship returns at most twenty objects, publishes no
    /// `@odata.nextLink` to page the rest and gives no count, so a cut list is indistinguishable
    /// from a complete one. The only way to be sure is to re-read the relationship's own
    /// endpoint, which is not capped - so every feed that expands one must declare how.
    #[test]
    fn the_owners_feeds_can_complete_a_capped_collection() -> Result<()> {
        let sources = load_m365_toml()?;
        let mut checked = 0;
        for group in sources.0.values() {
            for (name, source) in group {
                if !name.ends_with("_owners") {
                    continue;
                }
                checked += 1;
                assert_eq!(
                    source.explode.as_deref(),
                    Some("owners"),
                    "{name}: must explode owners so no single record can exceed maxchars"
                );
                let refetch = source.refetch_at_cap.as_deref().unwrap_or_else(|| {
                    panic!("{name}: must declare refetch_at_cap or a capped list ships short")
                });
                assert!(
                    refetch.contains("{id}"),
                    "{name}: refetch_at_cap must name the object: {refetch}"
                );
                assert!(
                    refetch.contains("/owners?$select=id,userPrincipalName,displayName"),
                    "{name}: refetch_at_cap must read owners and select the same fields as the \
                     expand, or a completed list has a different shape to a capped one: {refetch}"
                );
            }
        }
        assert_eq!(checked, 3, "expected three owners feeds");
        Ok(())
    }

    /// Builds a record with `count` owners, as `$expand` returns one.
    fn group_with_owners(count: usize) -> Value {
        json!({
            "id": "group-id",
            "displayName": "Infrastructure Admins",
            "owners": (0..count)
                .map(|n| json!({ "@odata.type": "#microsoft.graph.user", "id": n.to_string() }))
                .collect::<Vec<_>>(),
        })
    }

    /// A REFETCH THAT FAILED MUST NOT PASS FOR COMPLETE. The capped twenty are kept, because a
    /// short list is better than none, and the record is marked so every event says so. Driving
    /// the real function here rather than planting the marker by hand: deleting the insertion
    /// now fails this test, which it did not before.
    #[tokio::test]
    async fn a_failed_refetch_keeps_the_capped_list_and_flags_it_through_explosion() {
        let records = vec![group_with_owners(EXPANSION_CAP)];

        let completed = refetch_capped_collections(
            &records,
            "owners",
            "/beta/groups/{id}/owners",
            |_url| async { Err(anyhow!("Graph returned an error")) },
        )
        .await;

        assert_eq!(
            completed[0]["owners"]
                .as_array()
                .expect("still an array")
                .len(),
            EXPANSION_CAP,
            "the capped list is kept, not discarded"
        );
        assert_eq!(completed[0]["owners_incomplete"], json!(true));

        let events = explode_value(&completed[0], "owners");
        assert_eq!(events.len(), EXPANSION_CAP);
        for event in &events {
            assert_eq!(
                event["owners_incomplete"],
                json!(true),
                "every event from a failed refetch must carry the flag"
            );
        }
    }

    /// A SUCCESSFUL REFETCH REPLACES THE CAPPED LIST ON THE RIGHT RECORD AND ONLY THAT ONE.
    /// Two records, one at the cap and one below it: the capped one is completed, the other is
    /// untouched and costs no request. Neither is marked incomplete.
    #[tokio::test]
    async fn a_successful_refetch_replaces_only_the_capped_record() {
        let mut below = group_with_owners(3);
        below["id"] = json!("other-group-id");
        let records = vec![group_with_owners(EXPANSION_CAP), below];

        let requested = std::sync::Mutex::new(Vec::<String>::new());
        let completed =
            refetch_capped_collections(&records, "owners", "/beta/groups/{id}/owners", |url| {
                requested.lock().expect("lock").push(url);
                async {
                    Ok((0..31)
                        .map(|n| {
                            json!({
                                "@odata.type": "#microsoft.graph.servicePrincipal",
                                "id": format!("complete-{n}"),
                            })
                        })
                        .collect())
                }
            })
            .await;

        assert_eq!(
            requested.lock().expect("lock").as_slice(),
            ["/beta/groups/group-id/owners".to_owned()],
            "only the record at the cap is re-read"
        );
        assert_eq!(
            completed[0]["owners"].as_array().expect("array").len(),
            31,
            "the capped list is replaced by the complete one"
        );
        assert_eq!(completed[0]["owners"][0]["id"], json!("complete-0"));
        assert!(
            completed[0].get("owners_incomplete").is_none(),
            "a completed list must not be flagged"
        );
        assert_eq!(
            completed[1]["owners"].as_array().expect("array").len(),
            3,
            "the record below the cap is untouched"
        );
        assert_eq!(completed[1]["id"], json!("other-group-id"));
    }

    /// OWNERS ARE A MIXED COLLECTION: a user owner and a service principal owner, and the
    /// difference is the whole point - a service principal owning a privileged group is an
    /// ownership path with no interactive sign-in behind it. `@odata.type` is what separates
    /// them and it survives both the nested `$select` and the refetch, so it must survive
    /// explosion too. A service principal has no `userPrincipalName`, which must not break the
    /// event.
    #[test]
    fn an_exploded_mixed_owner_list_keeps_the_type_of_each_owner() {
        let group = json!({
            "id": "group-object-id",
            "displayName": "Infrastructure Admins",
            "owners": [
                {
                    "@odata.type": "#microsoft.graph.user",
                    "id": "user-owner",
                    "userPrincipalName": "a@example.gov.uk",
                    "displayName": "A",
                },
                {
                    "@odata.type": "#microsoft.graph.servicePrincipal",
                    "id": "sp-owner",
                    "displayName": "Some App",
                },
            ],
        });

        let events = explode_value(&group, "owners");

        assert_eq!(events.len(), 2);
        assert_eq!(events[0]["@odata.type"], json!("#microsoft.graph.user"));
        assert_eq!(events[0]["userPrincipalName"], json!("a@example.gov.uk"));
        assert_eq!(
            events[1]["@odata.type"],
            json!("#microsoft.graph.servicePrincipal"),
            "a service principal owner must be distinguishable from a user owner"
        );
        assert!(
            events[1].get("userPrincipalName").is_none(),
            "a service principal has no UPN and that must not invent one"
        );
        for event in &events {
            assert_eq!(event["parent_id"], json!("group-object-id"));
        }
    }

    /// THE GROUP REFETCH MUST STAY ON /beta. Measured 2026-10-28 on all six sampled groups
    /// with a service principal owner: /v1.0/groups/{id}/owners returned only the users, while
    /// /beta matched `$expand` exactly. Re-reading a capped group on v1.0 would therefore
    /// DELETE every service principal owner and mark the result complete. Applications and
    /// service principals have no such defect and stay on v1.0. Tidying this to v1.0 for
    /// consistency would silently lose ownership, so it is pinned here with the reason.
    #[test]
    fn the_group_refetch_is_beta_because_v1_omits_service_principal_owners() -> Result<()> {
        let sources = load_m365_toml()?;
        let mut checked = 0;
        for group in sources.0.values() {
            for (name, source) in group {
                let Some(refetch) = source.refetch_at_cap.as_deref() else {
                    continue;
                };
                checked += 1;
                let expected = if name == "group_owners" {
                    "/beta/"
                } else {
                    "/v1.0/"
                };
                assert!(
                    refetch.starts_with(expected),
                    "{name}: refetch_at_cap must start {expected}: {refetch}"
                );
            }
        }
        assert_eq!(checked, 3, "expected three feeds with a cap refetch");
        Ok(())
    }

    /// The count reaching the cap is the ONLY signal Graph gives, so that is the trigger: at or
    /// over the cap is assumed incomplete, below it is complete and must cost nothing. Pinned as
    /// a pure function because the alternative is a tenant with twenty owners on something.
    #[test]
    fn a_collection_is_refetched_only_at_the_cap() {
        let template = "/v1.0/groups/{id}/owners?$select=id,userPrincipalName,displayName";
        let owners = |count: usize| {
            json!({
                "id": "group-id",
                "owners": (0..count).map(|n| json!({"id": n.to_string()})).collect::<Vec<_>>(),
            })
        };

        assert_eq!(
            capped_refetch_url(&owners(EXPANSION_CAP), "owners", template).as_deref(),
            Some("/v1.0/groups/group-id/owners?$select=id,userPrincipalName,displayName"),
            "a list at the cap must be re-read in full"
        );
        assert_eq!(
            capped_refetch_url(&owners(EXPANSION_CAP - 1), "owners", template),
            None,
            "a list below the cap is complete and must not cost a request"
        );
        assert_eq!(
            capped_refetch_url(&json!({ "id": "group-id" }), "owners", template),
            None,
            "a record with no expanded collection is not capped"
        );
        assert_eq!(
            capped_refetch_url(&owners(EXPANSION_CAP + 1), "owners", template).as_deref(),
            Some("/v1.0/groups/group-id/owners?$select=id,userPrincipalName,displayName"),
            "if Graph ever returns more than the documented cap, still re-read it"
        );
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
