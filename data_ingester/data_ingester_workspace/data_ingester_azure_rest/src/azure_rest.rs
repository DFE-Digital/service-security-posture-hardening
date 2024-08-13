use azure_mgmt_authorization::package_2022_04_01::models::role_assignment_properties::PrincipalType;
use azure_mgmt_authorization::package_2022_04_01::{
    models::{RoleAssignment as SDKRoleAssignment, RoleDefinition as SDKRoleDefinition},
    Client as ClientAuthorization,
};
use azure_mgmt_subscription::package_2021_10::{
    models::Subscription, Client as ClientSubscription,
};

use anyhow::{Context, Result};
use azure_core::auth::TokenCredential;
use azure_identity::ClientSecretCredential;
use azure_identity::TokenCredentialOptions;
use dyn_fmt::AsStrFormatExt;
use futures::stream::StreamExt;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::Value;
use std::iter;
use std::{collections::HashMap, sync::Arc};
use tracing::error;
use url::Url;

use data_ingester_splunk::splunk::ToHecEvents;
use data_ingester_splunk::splunk::{get_ssphp_run, HecEvent};

pub struct AzureRest {
    pub(crate) credential: Arc<ClientSecretCredential>,
    subscriptions: Subscriptions,
}

impl AzureRest {
    pub async fn new(client_id: &str, client_secret: &str, tenant_id: &str) -> Result<Self> {
        let http_client = azure_core::new_http_client();
        let credential = Arc::new(ClientSecretCredential::new(
            http_client,
            tenant_id.to_owned(),
            client_id.to_owned(),
            client_secret.to_owned(),
            TokenCredentialOptions::default(),
        ));
        let mut s = Self {
            credential,
            subscriptions: Subscriptions { inner: vec![] },
        };
        s.subscriptions = s.azure_subscriptions().await?;
        Ok(s)
    }

    // TODO Remove pub
    pub async fn azure_subscriptions(&self) -> Result<Subscriptions> {
        let client = ClientSubscription::builder(self.credential.clone()).build()?;
        let mut stream = client.subscriptions_client().list().into_stream();
        let mut subscriptions = vec![];
        while let Some(item) = stream.next().await {
            for sub in item?.value {
                subscriptions.push(sub);
            }
        }
        Ok(Subscriptions {
            inner: subscriptions,
        })
    }

    pub fn subscriptions(&self) -> &Subscriptions {
        &self.subscriptions
    }

    pub async fn get_security_contacts(&self) -> Result<ReturnTypes> {
        let url_template = "https://management.azure.com/subscriptions/{}/providers/Microsoft.Security/securityContacts?api-version=2020-01-01-preview";
        let results = self
            .rest_request_subscription_iter(url_template, "azure_users")
            .await?;
        Ok(results)
    }

    pub async fn get_security_center_built_in(&self) -> Result<ReturnTypes> {
        let url_template = "https://management.azure.com/subscriptions/{}/providers/Microsoft.Authorization/policyAssignments/SecurityCenterBuiltIn?api-version=2021-06-01";
        let results = self
            .rest_request_subscription_iter(url_template, "azure_users")
            .await?;
        Ok(results)
    }

    // pub async fn get_microsoft_authorization_role_definitions(&self) -> Result<Vec<HecEvent>> {
    //     let url_template = "https://management.azure.com/subscriptions/{}/providers/Microsoft.Authorization/roleDefinitions?api-version=2017-05-01";
    //     let results = self.rest_request_subscription_iter(url_template).await?;
    //     Ok(results)
    // }

    // Azure Foundation V2.0.0 1.23
    pub async fn azure_role_definitions(&self) -> Result<RoleDefinitions> {
        let client = ClientAuthorization::builder(self.credential.clone()).build()?;
        let mut collection = HashMap::new();
        for sub in self.subscriptions.inner.iter() {
            let sub_id = sub.subscription_id.as_ref().context("no sub id")?;
            let scope = format!("/subscriptions/{}", sub_id);
            let mut stream = client.role_definitions_client().list(scope).into_stream();
            while let Some(results) = stream.next().await {
                for item in results?.value {
                    _ = collection.insert(
                        item.id
                            .as_ref()
                            .context("No ID on role definition")?
                            .to_owned(),
                        RoleDefinition(item),
                    );
                }
            }
        }
        Ok(RoleDefinitions { inner: collection })
    }

    pub async fn azure_role_assignments(&self) -> Result<RoleAssignments> {
        let client = ClientAuthorization::builder(self.credential.clone()).build()?;
        let mut collection = HashMap::new();
        for sub in self.subscriptions.inner.iter() {
            let sub_id = sub.subscription_id.as_ref().context("no sub id")?;
            let scope = format!("/subscriptions/{}", sub_id);
            let mut stream = client
                .role_assignments_client()
                .list_for_scope(scope)
                .into_stream();
            while let Some(results) = stream.next().await {
                for item in results?.value {
                    _ = collection.insert(
                        item.id
                            .as_ref()
                            .context("No ID on role assignment")?
                            .to_owned(),
                        RoleAssignment(item),
                    );
                }
            }
        }
        Ok(RoleAssignments { inner: collection })
    }

    // pub async fn azure_resource_graph(&self) -> Result<()> {
    //     let endpoint = "https://management.azure.com/providers/Microsoft.ResourceGraph/resources?api-version=2021-03-01";
    //     for sub in self.subscriptions.inner.iter() {
    //         let sub_id = sub.subscription_id.as_ref().context("no sub id")?;

    //         for table in &crate::resource_graph::RESOURCE_GRAPH_TABLES {
    //             let request_body = ResourceGraphRequest::new(sub_id, &format!("{}", &table));
    //             let response: crate::resource_graph::ResourceGraphResponse = self.post_rest_request(&endpoint, &request_body).await?;
    //             dbg!(&response);
    //         }
    //     }
    //     Ok(response)
    // }

    // pub async fn get_microsoft_authorization_role_assignments(&self) -> Result<Vec<HecEvent>> {
    //     let url_template = "https://management.azure.com/subscriptions/{}/providers/Microsoft.Authorization/roleassignments?api-version=2017-10-01-preview";
    //     let results = self.rest_request_subscription_iter(url_template).await?;
    //     Ok(results)
    // }

    pub async fn get_microsoft_security_settings(&self) -> Result<ReturnTypes> {
        let url_template = "https://management.azure.com/subscriptions/{}/providers/Microsoft.Security/settings?api-version=2021-06-01";
        let results = self
            .rest_request_subscription_iter(url_template, "azure_users")
            .await?;
        Ok(results)
    }

    pub async fn get_microsoft_security_auto_provisioning_settings(&self) -> Result<ReturnTypes> {
        let url_template = "https://management.azure.com/subscriptions/{}/providers/Microsoft.Security/autoProvisioningSettings?api-version=2017-08-01-preview";
        let results = self
            .rest_request_subscription_iter(url_template, "azure_users")
            .await?;
        Ok(results)
    }

    /// Probably needs to iterate through all tenancies
    /// Azure Foundations
    /// 1.25
    pub async fn get_microsoft_subscription_policies(&self) -> Result<SubscriptionPolicies> {
        let url = "https://management.azure.com/providers/Microsoft.Subscription/policies/default?api-version=2021-01-01-privatepreview";
        let results = self.get_rest_request(url).await?;
        Ok(results)
    }

    pub async fn get_microsoft_sql_encryption_protection(&self) -> Result<ReturnTypes> {
        let mut collection = ReturnTypes::default();
        let url_template = "https://management.azure.com/subscriptions/{}/providers/Microsoft.Sql/servers?api-version=2022-05-01-preview";
        let results = self
            .rest_request_subscription_iter_no_hec(url_template)
            .await?;

        for entry in results.iter() {
            match entry {
                ReturnType::Collection {
                    value,
                    next_link: _,
                } => {
                    for server in value.iter() {
                        if let Some(server_url) = server
                            .as_object()
                            .and_then(|o| o.get("id"))
                            .and_then(|id| id.as_str())
                        {
                            let url = format!(
                                "https://management.azure.com{}/encryptionProtector?api-version=2022-05-01-preview",
                                server_url);
                            let result = self.get_rest_request::<ReturnType>(&url).await?;
                            collection.collection.push(
                                result.into_return_type_wrapper(
                                    url.as_str().to_string(),
                                    "azure_users",
                                ),
                            );
                        }
                    }
                }
                _ => unreachable!(),
            };
        }

        Ok(collection)
    }

    pub async fn get_rest_request<T: DeserializeOwned + std::fmt::Debug>(
        &self,
        url: &str,
    ) -> Result<T> {
        let response = self
            .credential
            .get_token(&["https://management.azure.com/.default"])
            .await?;

        let response = reqwest::Client::new()
            .get(url)
            .header(
                "Authorization",
                format!("Bearer {}", response.token.secret()),
            )
            .send()
            .await?
            .text()
            .await?;

        let rt: T = serde_json::from_str(&response)?;
        Ok(rt)
    }

    pub async fn post_rest_request<T: DeserializeOwned + std::fmt::Debug, B: Serialize>(
        &self,
        url: &str,
        body: &B,
    ) -> Result<T> {
        let response = self
            .credential
            .get_token(&["https://management.azure.com/.default"])
            .await?;

        let body_json =
            serde_json::to_string(&body).context("Serialize HTTP request body to json")?;

        let response = reqwest::Client::new()
            .post(url)
            .header(
                "Authorization",
                format!("Bearer {}", response.token.secret()),
            )
            .header("Content-Type", "application/json")
            .body(body_json)
            .send()
            .await
            .context("make HTTP request")?
            .text()
            .await
            .context("Get body from HTTP response")?;

        let rt: T = match serde_json::from_str(&response) {
            Ok(obj) => obj,
            Err(err) => {
                error!("serde error: {:?}", &err);
                error!("serde error &response: {:?}", &response);
                anyhow::bail!("Failed to deserialize HTTP response");
            }
        };
        Ok(rt)
    }

    pub async fn rest_request_subscription_iter_no_hec(
        &self,
        url_template: &str,
    ) -> Result<Vec<ReturnType>> {
        let response = self
            .credential
            .get_token(&["https://management.azure.com/.default"])
            .await?;

        let mut collection = vec![];
        for sub in self.subscriptions.inner.iter() {
            let sub_id = sub.subscription_id.as_ref().context("no sub id")?;
            let url = Url::parse(&url_template.format(&[sub_id]))?;

            let response = reqwest::Client::new()
                .get(url.clone())
                .header(
                    "Authorization",
                    format!("Bearer {}", response.token.secret()),
                )
                .send()
                .await?
                .text()
                .await?;

            let rt: ReturnType = serde_json::from_str(&response)?;
            collection.push(rt);
        }
        Ok(collection)
    }

    pub async fn rest_request_subscription_iter(
        &self,
        url_template: &str,
        ssphp_run_key: &str,
    ) -> Result<ReturnTypes> {
        let response = self
            .credential
            .get_token(&["https://management.azure.com/.default"])
            .await?;
        let mut collection = ReturnTypes::default();
        for sub in self.subscriptions.inner.iter() {
            let sub_id = sub.subscription_id.as_ref().context("no sub id")?;
            let url = Url::parse(&url_template.format(&[sub_id]))?;
            let response = reqwest::Client::new()
                .get(url.clone())
                .header(
                    "Authorization",
                    format!("Bearer {}", response.token.secret()),
                )
                .send()
                .await?
                .text()
                .await?;

            let rt: ReturnType = serde_json::from_str(&response)?;

            collection
                .collection
                .push(rt.into_return_type_wrapper(url.as_str().to_string(), ssphp_run_key));
        }
        Ok(collection)
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
pub enum ReturnType {
    Collection {
        value: Vec<serde_json::Value>,
        #[serde(rename = "@odata.nextLink")]
        next_link: Option<String>,
    },
    Array(Vec<serde_json::Value>),
    Value(serde_json::Value),
}

// TODO Use ToHecEvents trait
impl ReturnType {
    pub fn to_hec_events(&self, source: &str, ssphp_run_key: &str) -> Result<Vec<HecEvent>> {
        let mut collection = vec![];
        match self {
            ReturnType::Collection { value, next_link } => {
                if let Some(_next_link) = next_link {
                    error!(
                        "`ReturnType::Collection shouldn't have a `next_link`: {:?}",
                        self
                    );
                    unimplemented!();
                }
                for v in value.iter() {
                    collection.push(HecEvent::new_with_ssphp_run(
                        &v,
                        //&url.as_str(),
                        // TODO
                        source,
                        v.as_object()
                            .context("value is not an object")?
                            .get("type")
                            .context("No key 'type'")?
                            .as_str()
                            .context("Type is not a str")?,
                        get_ssphp_run(ssphp_run_key),
                    )?);
                }
            }
            ReturnType::Array(vec) => {
                for v in vec.iter() {
                    collection.push(HecEvent::new_with_ssphp_run(
                        &v,
                        // TODO
                        source,
                        //&url.as_str(),
                        v.as_object()
                            .context("value is not an object")?
                            .get("type")
                            .context("No key 'type'")?
                            .as_str()
                            .context("Type is not a str")?,
                        get_ssphp_run(ssphp_run_key),
                    )?);
                }
            }
            ReturnType::Value(value) => {
                collection.push(HecEvent::new_with_ssphp_run(
                    &value,
                    // TODO
                    source,
                    //&url.as_str(),
                    value
                        .as_object()
                        .context("value is not an object")?
                        .get("type")
                        .context("No key 'type'")?
                        .as_str()
                        .context("Type is not a str")?,
                    get_ssphp_run(ssphp_run_key),
                )?);
            }
        };
        Ok(collection)
    }
    fn into_return_type_wrapper(self, source: String, ssphp_run_key: &str) -> ReturnTypeWrapper {
        ReturnTypeWrapper {
            collection: self,
            source,
            ssphp_run_key: ssphp_run_key.to_string(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct ReturnTypeWrapper {
    collection: ReturnType,
    source: String,
    ssphp_run_key: String,
}

impl ToHecEvents for &ReturnTypeWrapper {
    fn to_hec_events(&self) -> Result<Vec<HecEvent>> {
        self.collection
            .to_hec_events(&self.source, &self.ssphp_run_key)
    }

    type Item = ();

    fn source(&self) -> &str {
        unimplemented!()
    }

    fn sourcetype(&self) -> &str {
        unimplemented!()
    }
    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        unimplemented!()
    }

    fn ssphp_run_key(&self) -> &str {
        &self.ssphp_run_key
    }
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct ReturnTypes {
    collection: Vec<ReturnTypeWrapper>,
}

impl ToHecEvents for &ReturnTypes {
    fn to_hec_events(&self) -> Result<Vec<HecEvent>> {
        Ok(self
            .collection
            .iter()
            .flat_map(|c| c.to_hec_events())
            .flatten()
            .collect())
    }

    type Item = ();

    fn source(&self) -> &str {
        unimplemented!()
    }

    fn sourcetype(&self) -> &str {
        unimplemented!()
    }

    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        unimplemented!()
    }

    fn ssphp_run_key(&self) -> &str {
        unimplemented!()
    }
}

#[cfg(feature = "live_tests")]
impl ReturnTypes {
    #[allow(dead_code)]
    fn is_empty(&self) -> bool {
        self.collection.is_empty()
    }
}

#[cfg(feature = "live_tests")]
#[cfg(test)]
pub(crate) mod live_tests {
    use std::env;

    use crate::azure_rest::Subscriptions;
    use anyhow::{Context, Result};
    use data_ingester_splunk::splunk::{set_ssphp_run, Splunk, ToHecEvents};
    use data_ingester_supporting::keyvault::get_keyvault_secrets;

    use super::AzureRest;

    pub(crate) async fn setup() -> Result<(AzureRest, Splunk)> {
        let secrets = get_keyvault_secrets(
            &env::var("KEY_VAULT_NAME").expect("Need KEY_VAULT_NAME enviornment variable"),
        )
        .await
        .unwrap();

        let splunk = Splunk::new(
            secrets.splunk_host.as_ref().context("No value")?,
            secrets.splunk_token.as_ref().context("No value")?,
        )?;

        set_ssphp_run("default")?;

        let azure_rest = AzureRest::new(
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

        Ok((azure_rest, splunk))
    }

    #[tokio::test]
    async fn test_azureclient_list_subscriptions() -> Result<()> {
        let (azure_rest, _splunk) = setup().await?;
        let subscriptions: Subscriptions = azure_rest.azure_subscriptions().await?;
        assert!(!subscriptions.inner.is_empty());
        Ok(())
    }

    #[tokio::test]
    async fn test_azureclient_security_contacts() -> Result<()> {
        let (azure_rest, splunk) = setup().await?;
        let collection = azure_rest.get_security_contacts().await?;
        assert!(!collection.is_empty());
        splunk.send_batch((&collection).to_hec_events()?).await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_get_security_center_built_in() -> Result<()> {
        let (azure_rest, splunk) = setup().await?;
        let collection = azure_rest.get_security_center_built_in().await?;
        assert!(!collection.is_empty());
        splunk.send_batch((&collection).to_hec_events()?).await?;
        Ok(())
    }

    // 2.1.15
    #[tokio::test]
    async fn test_azureclient_get_microsoft_security_auto_provisioning_settings() -> Result<()> {
        let (azure_rest, splunk) = setup().await?;
        let collection = azure_rest
            .get_microsoft_security_auto_provisioning_settings()
            .await?;
        splunk.send_batch((&collection).to_hec_events()?).await?;
        assert!(!collection.is_empty());
        Ok(())
    }

    // 2.1.21
    // 2.1.22
    #[tokio::test]
    async fn test_azureclient_get_microsoft_security_settings() -> Result<()> {
        let (azure_rest, splunk) = setup().await?;
        let collection = azure_rest.get_microsoft_security_settings().await?;
        splunk.send_batch((&collection).to_hec_events()?).await?;
        assert!(!collection.is_empty());
        Ok(())
    }

    // 4.1.3
    #[ignore]
    #[tokio::test]
    async fn test_azureclient_get_microsoft_sql_encryption_protection() -> Result<()> {
        let (azure_rest, splunk) = setup().await?;
        let collection = azure_rest.get_microsoft_sql_encryption_protection().await?;
        splunk.send_batch((&collection).to_hec_events()?).await?;
        assert!(!collection.is_empty());
        Ok(())
    }

    #[tokio::test]
    async fn test_azureclient_azure_role_definitions() -> Result<()> {
        let (azure_rest, splunk) = setup().await?;
        let collection = azure_rest.azure_role_definitions().await?;
        splunk.send_batch((&collection).to_hec_events()?).await?;
        assert!(!collection.inner.is_empty());
        Ok(())
    }

    #[tokio::test]
    async fn test_azureclient_azure_role_assignments() -> Result<()> {
        let (azure_rest, splunk) = setup().await?;
        let collection = azure_rest.azure_role_assignments().await?;
        splunk.send_batch((&collection).to_hec_events()?).await?;
        assert!(!collection.inner.is_empty());
        Ok(())
    }

    #[tokio::test]
    async fn test_get_microsoft_subscription_policies() -> Result<()> {
        let (azure_rest, splunk) = setup().await?;
        let collection = azure_rest.get_microsoft_subscription_policies().await?;
        splunk.send_batch((&collection).to_hec_events()?).await?;
        Ok(())
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RoleAssignment(SDKRoleAssignment);

impl RoleAssignment {
    pub fn role_definition_id(&self) -> Option<&String> {
        Some(&self.0.properties.as_ref()?.role_definition_id)
    }

    pub fn principal_type(&self) -> Option<&PrincipalType> {
        self.0.properties.as_ref()?.principal_type.as_ref()
    }

    pub fn principal_id(&self) -> Option<&String> {
        Some(&self.0.properties.as_ref()?.principal_id)
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RoleAssignments {
    pub inner: HashMap<String, RoleAssignment>,
}

impl ToHecEvents for &RoleAssignments {
    type Item = RoleAssignment;
    fn source(&self) -> &str {
        "azure_rest"
    }

    fn sourcetype(&self) -> &str {
        "SSPHP.azure.role_assignment"
    }
    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        Box::new(self.inner.values())
    }
    fn ssphp_run_key(&self) -> &str {
        "azure_resource_graph"
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RoleDefinition(SDKRoleDefinition);
impl RoleDefinition {
    pub fn role_name(&self) -> Option<&String> {
        self.0.properties.as_ref()?.role_name.as_ref()
    }

    pub fn id(&self) -> Option<&String> {
        self.0.id.as_ref()
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RoleDefinitions {
    pub inner: HashMap<String, RoleDefinition>,
}

impl ToHecEvents for &RoleDefinitions {
    type Item = RoleDefinition;
    fn source(&self) -> &str {
        "azure_rest"
    }

    fn sourcetype(&self) -> &str {
        "SSPHP.azure.role_definitions"
    }
    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        Box::new(self.inner.values())
    }
    fn ssphp_run_key(&self) -> &str {
        "azure_resource_graph"
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Subscriptions {
    pub(crate) inner: Vec<Subscription>,
}

impl ToHecEvents for &Subscriptions {
    type Item = Subscription;
    fn source(&self) -> &str {
        "azure_rest"
    }

    fn sourcetype(&self) -> &str {
        "SSPHP.azure.subscriptions"
    }

    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        Box::new(self.inner.iter())
    }
    fn ssphp_run_key(&self) -> &str {
        "azure_resource_graph"
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SubscriptionPolicies {
    #[serde(flatten)]
    inner: Value,
}

impl ToHecEvents for &SubscriptionPolicies {
    type Item = Self;
    fn source(&self) -> &str {
        "azure_rest"
    }

    fn sourcetype(&self) -> &str {
        "SSPHP.azure.subscription_policies"
    }

    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        Box::new(iter::once(self))
    }
    fn ssphp_run_key(&self) -> &str {
        "azure_resource_graph"
    }
}
