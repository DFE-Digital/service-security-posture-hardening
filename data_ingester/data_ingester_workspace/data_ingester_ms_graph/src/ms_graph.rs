use crate::conditional_access_policies::ConditionalAccessPolicies;
use crate::conditional_access_policies::ConditionalAccessPolicy;
use crate::msgraph_data::load_m365_toml;
use crate::role_assignment_schedule::RoleSchedules;
use crate::roles::RoleDefinitions;
use crate::service_principals::ServicePrincipals;
use crate::service_principals::ServicePrincipalsMap;
use crate::users::Users;
use crate::users::UsersMap;
use anyhow::{Context, Result};
use data_ingester_splunk::splunk::try_collect_send;
use data_ingester_splunk::splunk::ToHecEvents;
use data_ingester_splunk::splunk::{set_ssphp_run, Splunk};
use data_ingester_supporting::dns::resolve_txt_record;
use data_ingester_supporting::keyvault::Secrets;
use futures::StreamExt;
use graph_http::api_impl::RequestComponents;
use graph_http::api_impl::RequestHandler;
use graph_oauth::ClientSecretCredential;
use graph_oauth::ConfidentialClientApplication;
use graph_rs_sdk::header::HeaderMap;
use graph_rs_sdk::header::HeaderValue;
use graph_rs_sdk::header::CONTENT_TYPE;
use graph_rs_sdk::http::Method;
use graph_rs_sdk::Graph;
use graph_rs_sdk::GraphClient;
use graph_rs_sdk::GraphClientConfiguration;
use graph_rs_sdk::ODataQuery;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;
use std::env;
use std::fmt::Debug;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc::UnboundedSender;
use tracing::{error, info};

/// Recognise a Graph OData error body that the paging stream handed back as if it were data.
///
/// Graph reports a failed request as `{"error":{"code":..,"message":..}}` and NOTHING else, so the
/// test is deliberately narrow: exactly one value, an object carrying `error`, and no `value` key
/// of its own. A legitimate payload that merely happens to contain a field called `error`
/// alongside its data is not mistaken for a failure.
fn graph_odata_error(values: &[Value]) -> Option<String> {
    let [only] = values else {
        return None;
    };
    if only.get("value").is_some() {
        return None;
    }
    let error = only.get("error")?.as_object()?;
    let code = error
        .get("code")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let message = error.get("message").and_then(Value::as_str).unwrap_or("");
    Some(format!("{code}: {message}"))
}

/// A Client for Ms Graph
#[derive(Clone)]
pub struct MsGraph {
    client: Graph,
    beta_client: Graph,
    client_application: ConfidentialClientApplication<ClientSecretCredential>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(untagged)]
pub enum MsGraphGetResponse {
    Collection { value: Vec<serde_json::Value> },
    Single(serde_json::Value),
}

impl MsGraph {
    pub async fn new(client_id: &str, client_secret: &str, tenant_id: &str) -> Result<MsGraph> {
        let client_application = ConfidentialClientApplication::builder(client_id)
            .with_client_secret(client_secret)
            .with_tenant(tenant_id)
            .build();

        let client_config = GraphClientConfiguration::new()
            .client_application(client_application.clone())
            .timeout(Duration::from_secs(30))
            .retry(Some(10))
            .wait_for_retry_after_headers(true);

        let client = GraphClient::from(client_config.clone());

        let mut beta_client = GraphClient::from(client_config.clone());
        beta_client.use_beta();

        Ok(MsGraph {
            client,
            beta_client,
            client_application,
        })
    }

    /// `url` must start with the API version segment, `/v1.0/...` or `/beta/...`.
    /// The base URL's path is joined per RFC 3986, so a leading `/` replaces it
    /// entirely and the version is lost; Graph then reads the first path segment
    /// as the version and returns an error document rather than data.
    pub async fn get_url(&self, url: &str) -> Result<Vec<Value>> {
        let current_client: graph_http::api_impl::Client = graph_http::api_impl::Client::builder()
            .client_application(self.client_application.clone())
            .retry(Some(20))
            .wait_for_retry_after_headers(true)
            .into();

        let mut header_map = HeaderMap::new();

        _ = header_map
            .entry(CONTENT_TYPE)
            .or_insert(HeaderValue::from_static("application/json"));

        let full_url = self.client.url().join(url)?;

        let request_components = RequestComponents::new(
            graph_core::resource::ResourceIdentity::Custom,
            full_url,
            Method::GET,
        );

        let request_handler =
            RequestHandler::new(current_client.clone(), request_components, None, None)
                .headers(header_map);

        let mut stream = request_handler.paging().stream::<MsGraphGetResponse>()?;

        let mut collection = Vec::default();
        while let Some(result) = stream.next().await {
            let response = result?;
            let body = response.into_body()?;

            match body {
                MsGraphGetResponse::Collection { value } => collection.extend(value),
                MsGraphGetResponse::Single(value) => collection.push(value),
            }
        }

        // A FAILED REQUEST MUST NOT LOOK LIKE AN EMPTY FEED. The paging stream hands a Graph
        // OData error body back as an ordinary single value, so a 400 arrived here as a one-row
        // "collection" and every caller treated it as data. That is how the role eligibility feed
        // returned nothing for as long as it existed while looking merely empty - see the comment
        // on `list_role_eligibility_schedules`. Refusing it here rather than in one caller means
        // no other endpoint can fail the same quiet way.
        if let Some(error) = graph_odata_error(&collection) {
            error!(
                url = url,
                error = error.as_str(),
                "GRAPH_REQUEST_FAILED: Graph returned an error body instead of data"
            );
            anyhow::bail!("Graph returned an error for {url}: {error}");
        }

        Ok(collection)
    }

    pub async fn get_transitive_members_of_for_users(
        &self,
        users: &mut UsersMap<'_>,
    ) -> Result<()> {
        let mut tasks = vec![];
        for user in users.inner.values_mut() {
            if !user.account_enabled.unwrap_or_default() {
                continue;
            }

            let task = async {
                let url = format!("/v1.0/users/{}/transitiveMemberOf", user.id);
                let transitive_member_of = match self.get_url(&url).await {
                    Ok(result) => result,
                    Err(error) => {
                        error!(error=?error, url=?url, "Failed to get Group transitiveMemberOf");
                        return Err(error);
                    }
                };
                user.transitive_member_of = transitive_member_of
                    .into_iter()
                    .filter_map(|membership| {
                        match serde_json::from_value(membership.clone()) {
                            Ok(result) => Some(result),
                            Err(err) => {
                                error!(error=?err, value=?membership, "Failed to deserialize Value to GroupOrRole");
                                None
                            }
                        }
                    })
                    .collect();
                Ok(())
            };
            tasks.push(task);
        }
        let stream = futures::stream::iter(tasks).buffer_unordered(10);
        let _results = stream.collect::<Vec<_>>().await;

        Ok(())
    }

    /// The service principal equivalent of `get_transitive_members_of_for_users`.
    ///
    /// WHY A PER-PRINCIPAL CALL AND NOT `$expand`. The collection-level call below does carry
    /// `$expand=transitiveMemberOf`, exactly as the user stream does, and for the same reason it
    /// is not enough on its own: GRAPH CAPS EXPANDED COLLECTIONS, so a principal with more
    /// memberships than the cap comes back truncated and silently looks less privileged than it
    /// is. That is why the user path does both, and this one has to as well. Confirmed by Alex,
    /// 2026-10-14.
    ///
    /// COST, stated rather than buried: one call per enabled service principal, about 14,000 in
    /// this tenant against the roughly 20,000 the user path already makes, at the same
    /// concurrency of 10. The alternative considered was a group-side fetch -
    /// `/v1.0/groups/{id}/transitiveMembers` for the ~40 role-assignable groups, 40 calls for the
    /// same answer - which is far cheaper but needs machinery that does not exist, where this
    /// mirrors code that is already proven in production.
    ///
    /// Disabled principals are skipped, the same rule the user path applies, so the enabled
    /// filter happens at collection time rather than in every search downstream.
    ///
    /// Errors are logged per principal and NOT propagated, matching the user path: one service
    /// principal that cannot be read must not abandon the other 14,000. A principal whose call
    /// failed keeps `transitive_member_of` unset, which reads as "no memberships known" rather
    /// than "no memberships" - the distinction matters when this is used to build a privileged
    /// population, so an unset value must never be treated as an empty one.
    pub async fn get_transitive_members_of_for_service_principals(
        &self,
        service_principals: &mut ServicePrincipalsMap,
    ) -> Result<()> {
        let mut tasks = vec![];
        for service_principal in service_principals.inner.values_mut() {
            if !service_principal.account_enabled.unwrap_or_default() {
                continue;
            }

            let task = async {
                let url = format!(
                    "/v1.0/servicePrincipals/{}/transitiveMemberOf",
                    service_principal.id
                );
                let transitive_member_of = match self.get_url(&url).await {
                    Ok(result) => result,
                    Err(error) => {
                        error!(error=?error, url=?url, "Failed to get ServicePrincipal transitiveMemberOf");
                        return Err(error);
                    }
                };
                service_principal.transitive_member_of = transitive_member_of
                    .into_iter()
                    .filter_map(|membership| {
                        match serde_json::from_value(membership.clone()) {
                            Ok(result) => Some(result),
                            Err(err) => {
                                error!(error=?err, value=?membership, "Failed to deserialize Value to GroupOrRole");
                                None
                            }
                        }
                    })
                    .collect();
                Ok(())
            };
            tasks.push(task);
        }
        let stream = futures::stream::iter(tasks).buffer_unordered(10);
        let _results = stream.collect::<Vec<_>>().await;

        Ok(())
    }

    /// PIM ELIGIBILITY: the roles a principal may activate, not the ones it holds. These are the
    /// rows that carry the 38 role-assignable groups, so a principal's privilege-through-a-group
    /// depends entirely on them arriving.
    ///
    /// Deserialising is delegated to `RoleSchedules::from_values`, which PARTITIONS and logs every
    /// rejected row in full. Both feeds used to do it inline with a `filter_map` that dropped
    /// failures, which is how this feed was lost in silence for as long as it has been collected.
    pub async fn list_role_eligibility_schedules(&self) -> Result<RoleSchedules> {
        // NO `activatedUsing` IN THIS EXPAND. It exists on unifiedRoleAssignmentScheduleInstance
        // and NOT on unifiedRoleEligibilityScheduleInstance, so asking for it made Graph reject
        // the whole request with HTTP 400:
        //
        //   Parsing OData Select and Expand failed: Could not find a property named
        //   'activatedUsing' on type 'microsoft.graph.unifiedRoleEligibilityScheduleInstance'.
        //
        // This call therefore returned NOTHING for as long as it has existed, and because
        // `get_url` handed the error body back as if it were a one-row collection, it looked like
        // a feed that was simply empty. The consequence reached all the way to the detections: no
        // principal was ever recorded as holding a directory role through a group, because the 38
        // role-assignable groups live in this feed. Found 2026-10-15 from the rejected-row logging
        // in `RoleSchedules::from_values`, which printed the raw 400 body on its first run.
        //
        // The expand below matches the `role_eligibility_schedule_instances` entry in
        // ms_graph.toml, which has always been correct - which is why Splunk held 54 rows from the
        // generic collector while this call returned none.
        let result = self.get_url("/beta/roleManagement/directory/roleEligibilityScheduleInstances?$expand=appScope,directoryScope,principal,roleDefinition").await?;
        Ok(RoleSchedules::from_values(
            "roleEligibilityScheduleInstances",
            result,
        ))
    }

    /// PIM ASSIGNMENTS: the roles a principal actually holds, standing or activated.
    ///
    /// Same partitioning and logging as the eligibility feed above, and deliberately through the
    /// same function so the two cannot drift apart again.
    pub async fn list_role_assignment_schedules(&self) -> Result<RoleSchedules> {
        let result = self.get_url("/beta/roleManagement/directory/roleAssignmentScheduleInstances?$expand=appScope,directoryScope,principal,roleDefinition").await?;
        Ok(RoleSchedules::from_values(
            "roleAssignmentScheduleInstances",
            result,
        ))
    }

    // /// M365 V2 1.1.18
    // /// This does not work - No such API
    // pub async fn get_app_family_details(&self) -> Result<AppFamilyDetails> {
    //     let result = self
    //         .get(&self.beta_client, "/organization/getAppFamilyDetails")
    //         .await?;
    //     Ok(AppFamilyDetails { inner: result })
    // }

    pub async fn list_conditional_access_policies(&self) -> Result<ConditionalAccessPolicies> {
        let mut stream = self
            .client
            .identity()
            .list_policies()
            .paging()
            .stream::<ConditionalAccessPolicies>()?;

        let mut caps = ConditionalAccessPolicies::new();

        while let Some(result) = stream.next().await {
            let response = result?;
            let body = response.into_body()?;
            let _ = body
                .inner
                .into_iter()
                .map(|mut cap: ConditionalAccessPolicy| {
                    cap.has_untrusted_conditions = Some(cap.has_other_conditions());
                    cap
                })
                .collect_into(&mut caps.inner);
        }
        Ok(caps)
    }

    pub async fn list_role_definitions(&self) -> Result<RoleDefinitions> {
        let mut stream = self
            .beta_client
            .custom(Method::GET, None)
            .extend_path(&["roleManagement", "directory", "roleDefinitions"])
            .paging()
            .stream::<RoleDefinitions>()?;

        let mut roles = RoleDefinitions::new();
        while let Some(result) = stream.next().await {
            let response = result?;

            let body = response.into_body()?;

            roles.value.extend(body.value)
        }
        Ok(roles)
    }

    pub async fn list_users_channel(&self, sender: UnboundedSender<UsersMap<'_>>) -> Result<()> {
        let mut stream = self
            .beta_client
            .users()
            .list_user()
            .select(&[
                "accountEnabled",
                "assignedPlans",
                "description",
                "displayName",
                "givenName",
                "id",
                "mail",
                "onPremisesSamAccountName",
                "onPremisesSyncEnabled",
                "surname",
                "transitiveMemberOf",
                "userPrincipalName",
                "userType",
                // TODO Needs AuditLog.Read.All MS Graph permission
                // "signInActivity",
            ])
            .expand(&["transitiveMemberOf"])
            .top("999")
            .paging()
            .stream::<Users>()?;

        let mut total_users = 0;
        let mut batch = 1;

        while let Some(result) = stream.next().await {
            let response = result?;

            let mut users = response.into_body()?;

            total_users += users.value.len();

            info!(
                "Getting users batch {}, total users: {}",
                batch, total_users
            );
            batch += 1;

            users.value.iter_mut().for_each(|u| {
                u.assigned_plans_remove_deleted();
            });

            let mut users_map = UsersMap::default();

            users_map
                .extend_from_users(users)
                .context("unable to extend users")?;

            sender
                .send(users_map)
                .expect("Unable to send Users to channel");
        }

        Ok(())
    }

    /// The service principal equivalent of `list_users_channel`.
    ///
    /// Streams service principals into a channel in pages of 999 so the consumer can enrich one
    /// batch while the next is still being fetched, which is what the user path does.
    ///
    /// THE `$select` IS DELIBERATE, NOT LAZINESS, and is the same list the `service_principals`
    /// entry in ms_graph.toml uses. A full servicePrincipal carries `appRoles` and
    /// `oauth2PermissionScopes`, which for a large application run to tens of thousands of
    /// characters; Splunk stops extracting JSON fields after `[kv] maxchars`, 10,240 by default,
    /// so an oversized record arrives whole and yields NO fields at all - measured on the user
    /// feed as 23.8% of records. Extend this list a few properties at a time once the feed is
    /// proven: ONE PROPERTY GRAPH DOES NOT RECOGNISE FAILS THE WHOLE CALL with BadRequest.
    ///
    /// `transitiveMemberOf` is both selected and expanded here, as on the user stream, but it is
    /// NOT sufficient on its own - Graph caps expanded collections, so
    /// `get_transitive_members_of_for_service_principals` has to run over the result as well.
    pub async fn list_service_principals_channel(
        &self,
        sender: UnboundedSender<ServicePrincipalsMap>,
    ) -> Result<()> {
        let mut stream = self
            .beta_client
            .service_principals()
            .list_service_principal()
            .select(&[
                "accountEnabled",
                "appId",
                "appOwnerOrganizationId",
                "appRoleAssignmentRequired",
                "displayName",
                "id",
                "publisherName",
                "servicePrincipalType",
                "signInAudience",
                "tags",
                "transitiveMemberOf",
            ])
            .expand(&["transitiveMemberOf"])
            .top("999")
            .paging()
            .stream::<ServicePrincipals>()?;

        let mut total_service_principals = 0;
        let mut batch = 1;

        while let Some(result) = stream.next().await {
            let response = result?;

            let service_principals = response.into_body()?;

            total_service_principals += service_principals.value.len();

            info!(
                "Getting service principals batch {}, total service principals: {}",
                batch, total_service_principals
            );
            batch += 1;

            let mut service_principals_map = ServicePrincipalsMap::default();

            service_principals_map
                .extend_from_service_principals(service_principals)
                .context("unable to extend service principals")?;

            sender
                .send(service_principals_map)
                .expect("Unable to send ServicePrincipals to channel");
        }

        Ok(())
    }

    pub async fn get_domains(&self) -> Result<Domains> {
        let response = self.client.domains().list_domain().send().await?;
        let mut body: Domains = response.json().await?;
        for domain in body.inner.iter_mut() {
            if let Ok(txt) = resolve_txt_record(&domain.id).await {
                domain.txt_records = Some(txt);
            }

            let dmarc_domain = format!("_dmarc.{}", domain.id);
            if let Ok(dmarc) = resolve_txt_record(&dmarc_domain).await {
                domain.dmarc = Some(dmarc);
            }
        }
        Ok(body)
    }

    #[allow(dead_code)]
    pub async fn list_token_lifetime_policies(&self) -> Result<Value> {
        let response = self
            .client
            .policies()
            .list_token_lifetime_policies()
            .send()
            .await?;
        let body = response.json().await?;
        Ok(body)
    }
}

// #[derive(Debug, Serialize, Deserialize, Default)]
// pub struct AppFamilyDetails {
//     #[serde(flatten)]
//     inner: Vec<Value>,
// }

// impl ToHecEvents for &AppFamilyDetails {
//     type Item = Value;
//     fn source(&self) -> &str {
//         "msgraph"
//     }

//     fn sourcetype(&self) -> &str {
//         "m365:app_family_details"
//     }

//     fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
//         Box::new(self.inner.iter())
//     }
// }

/// CIS Azure 365 Azure 1.4
/// CIS Azure 365 Azure 4.8
/// CIS Azure 365 Azure 4.9
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Domains {
    #[serde(rename = "@odata.context")]
    pub odata_context: String,
    #[serde(rename = "value")]
    pub inner: Vec<Domain>,
}

/// CIS Azure 365 Azure 1.4
/// CIS Azure 365 Azure 4.8
/// CIS Azure 365 Azure 4.9
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Domain {
    pub authentication_type: String,
    pub availability_status: Option<String>,
    pub id: String,
    pub is_admin_managed: bool,
    pub is_default: bool,
    pub is_initial: bool,
    pub is_root: bool,
    pub is_verified: bool,
    pub password_notification_window_in_days: Option<i32>,
    pub password_validity_period_in_days: Option<i32>,
    pub state: Value,
    pub supported_services: Vec<String>,
    pub txt_records: Option<Vec<String>>,
    pub dmarc: Option<Vec<String>>,
}

impl ToHecEvents for &Domains {
    type Item = Domain;
    fn source(&self) -> &str {
        "/v1.0/domains"
    }

    fn sourcetype(&self) -> &str {
        "ssphp:ms_graph:json"
    }

    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        Box::new(self.inner.iter())
    }
    fn ssphp_run_key(&self) -> &str {
        "m365"
    }
}

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Group {
    pub id: String,
    // pub deleted_date_time: Value,
    // pub classification: Value,
    // pub created_date_time: String,
    // pub creation_options: Vec<Value>,
    // pub description: Value,
    pub display_name: Option<String>,
    // pub expiration_date_time: Value,
    // pub group_types: Vec<String>,
    // pub is_assignable_to_role: Value,
    // pub mail: String,
    // pub mail_enabled: bool,
    // pub mail_nickname: String,
    // pub membership_rule: Value,
    // pub membership_rule_processing_state: Value,
    // pub on_premises_domain_name: Value,
    // pub on_premises_last_sync_date_time: Value,
    // pub on_premises_net_bios_name: Value,
    // pub on_premises_sam_account_name: Value,
    // pub on_premises_security_identifier: Value,
    // pub on_premises_sync_enabled: Value,
    // pub preferred_data_location: Value,
    // pub preferred_language: Value,
    // pub proxy_addresses: Vec<String>,
    // pub renewed_date_time: String,
    // pub resource_behavior_options: Vec<Value>,
    // pub resource_provisioning_options: Vec<Value>,
    pub security_enabled: Option<bool>,
    pub security_identifier: Option<String>,
    // pub theme: Value,3
    pub visibility: Option<String>,
    // pub on_premises_provisioning_errors: Vec<Value>,
    // pub service_provisioning_errors: Vec<Value>,
}

pub async fn m365(secrets: Arc<Secrets>, splunk: Arc<Splunk>) -> Result<()> {
    let _ = set_ssphp_run(crate::SSPHP_RUN_KEY)?;

    info!("Starting M365 collection");
    info!("GIT_HASH: {}", env!("GIT_HASH"));

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

    info!("MS Graph logged in");

    let sources = load_m365_toml()?;

    info!("Loaded {} m365 sources", sources.len());
    sources
        .process_sources(&ms_graph, &splunk, crate::SSPHP_RUN_KEY)
        .await?;

    let _ = try_collect_send("MS Graph Domains", ms_graph.get_domains(), &splunk).await;

    info!("M365 Collection Complete");

    Ok(())
}

#[cfg(test)]
mod graph_odata_error_tests {
    use super::graph_odata_error;
    use serde_json::json;

    /// THE EXACT BODY that made the role eligibility feed look empty, captured from the live run
    /// on 2026-10-15. Before this, it was pushed into the collection as a single row and every
    /// caller read it as data.
    #[test]
    fn the_real_badrequest_body_is_recognised() {
        let body = json!({
            "error": {
                "code": "BadRequest",
                "innerError": {
                    "client-request-id": "cb59e572-8c00-4c97-9cef-128df8e5180f",
                    "date": "2026-10-15T11:29:45",
                    "request-id": "cb59e572-8c00-4c97-9cef-128df8e5180f"
                },
                "message": "Parsing OData Select and Expand failed: Could not find a property named 'activatedUsing' on type 'microsoft.graph.unifiedRoleEligibilityScheduleInstance'."
            }
        });

        let detected = graph_odata_error(&[body]).expect("a Graph error body must be recognised");

        assert!(detected.starts_with("BadRequest: "), "{detected}");
        assert!(detected.contains("activatedUsing"), "{detected}");
    }

    /// Real data must never be mistaken for a failure.
    #[test]
    fn ordinary_rows_are_not_an_error() {
        let rows = vec![
            json!({ "id": "a", "displayName": "One" }),
            json!({ "id": "b", "displayName": "Two" }),
        ];
        assert!(graph_odata_error(&rows).is_none());
    }

    /// A single ordinary row is not an error either - the test is the `error` key, not the count.
    #[test]
    fn a_single_ordinary_row_is_not_an_error() {
        assert!(graph_odata_error(&[json!({ "id": "a" })]).is_none());
    }

    /// An empty collection is a genuinely empty feed, not a failure.
    #[test]
    fn an_empty_collection_is_not_an_error() {
        assert!(graph_odata_error(&[]).is_none());
    }

    /// A payload that carries its own data alongside a field called `error` is data. The test is
    /// deliberately narrow so a legitimate record is never refused.
    #[test]
    fn a_payload_with_both_value_and_error_is_data() {
        let body = json!({
            "value": [{ "id": "a" }],
            "error": { "code": "Partial", "message": "some rows were skipped" }
        });
        assert!(graph_odata_error(&[body]).is_none());
    }

    /// An error body missing code or message is still an error, reported with what is there.
    #[test]
    fn a_sparse_error_body_is_still_recognised() {
        let detected =
            graph_odata_error(&[json!({ "error": { "code": "Throttled" } })]).expect("recognised");
        assert_eq!(detected, "Throttled: ");
    }
}

#[cfg(feature = "live_tests")]
#[cfg(test)]
pub(crate) mod live_tests {
    use std::env;

    use super::MsGraph;
    use crate::users::UsersMap;

    use anyhow::{Context, Result};
    use data_ingester_splunk::splunk::SplunkTrait;
    use data_ingester_splunk::splunk::{set_ssphp_run, Splunk, ToHecEvents};
    use data_ingester_supporting::keyvault::get_keyvault_secrets;

    pub async fn setup() -> Result<(Splunk, MsGraph)> {
        let secrets = get_keyvault_secrets(&env::var("KEY_VAULT_NAME")?).await?;

        let _ = set_ssphp_run("default")?;
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

        let splunk = Splunk::new(
            secrets.splunk_host.as_ref().context("No value")?,
            secrets.splunk_token.as_ref().context("No value")?,
            true,
        )?;

        Ok((splunk, ms_graph))
    }

    #[tokio::test]
    async fn get_users_channel() -> Result<()> {
        let (splunk, ms_graph) = setup().await?;

        let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel::<UsersMap>();

        let ms_graph_clone = ms_graph.clone();
        let list_users = tokio::spawn(async move {
            ms_graph_clone.list_users_channel(sender).await?;
            anyhow::Ok::<()>(())
        });

        let mut users_map = UsersMap::default();
        while let Some(users) = receiver.recv().await {
            users_map.extend(users);
        }
        let _ = list_users.await?;

        assert!(!users_map.inner.is_empty());
        splunk.send_batch((&users_map).to_hec_events()?).await?;
        Ok(())
    }

    #[tokio::test]
    async fn list_conditional_access_policies() -> Result<()> {
        let (splunk, ms_graph) = setup().await?;
        let caps = ms_graph.list_conditional_access_policies().await?;
        let hec_events = (&caps).to_hec_events()?;
        splunk.send_batch(hec_events).await?;
        Ok(())
    }

    #[tokio::test]
    async fn get_domains() -> Result<()> {
        let (splunk, ms_graph) = setup().await?;
        let domains = ms_graph.get_domains().await?;
        splunk.send_batch((&domains).to_hec_events()?).await?;
        Ok(())
    }

    // #[ignore]
    // #[tokio::test]
    // async fn list_token_lifetime_policies() -> Result<()> {
    //     let (splunk, ms_graph) = setup().await?;
    //     let result = ms_graph.list_token_lifetime_policies().await?;
    //     let hec = HecDynamic::new(result, "msgraph:tokenlifetime", "aktest");
    //     splunk.send_batch((&hec).to_hec_events()?).await?;
    //     Ok(())
    // }

    #[tokio::test]
    async fn list_role_eligibility_schedules() -> Result<()> {
        let (splunk, ms_graph) = setup().await?;
        let result = ms_graph.list_role_eligibility_schedules().await?;
        assert!(result.inner.len() > 2);
        Ok(())
    }

    #[tokio::test]
    async fn list_role_assignment_schedules() -> Result<()> {
        let (splunk, ms_graph) = setup().await?;
        let result = ms_graph.list_role_assignment_schedules().await?;
        //splunk.send_batch((&result).to_hec_events()?).await?;
        assert!(result.inner.len() > 2);
        Ok(())
    }

    #[tokio::test]
    async fn list_role_definitions() -> Result<()> {
        let (splunk, ms_graph) = setup().await?;
        let result = ms_graph.list_role_definitions().await?;
        splunk.send_batch((&result).to_hec_events()?).await?;
        Ok(())
    }

    // API only exists in an undocumented preview state
    // #[ignore]
    // #[tokio::test]
    // async fn get_app_family_details() -> Result<()> {
    //     let (splunk, ms_graph) = setup().await?;
    //     let result = ms_graph.get_app_family_details().await?;
    //     splunk.send_batch((&result).to_hec_events()?).await?;
    //     Ok(())
    // }
}
