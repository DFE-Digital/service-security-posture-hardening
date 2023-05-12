use anyhow::Result;
use azure_core::ClientOptions;
use azure_identity::ClientSecretCredential;
use azure_mgmt_security::package_composite_v3::Client;
use azure_mgmt_subscription::models::Subscription;
use futures::{StreamExt, TryStreamExt};
use modular_input::Event;

use std::sync::mpsc::Sender;
use std::sync::Arc;

use std::fmt;
use std::ops::Deref;
use tracing::instrument;

#[derive(Clone)]
pub struct AzureClient {
    credentials: Arc<ClientSecretCredential>,
}

#[derive(Clone)]
pub struct SubscriptionId(String);

impl Deref for SubscriptionId {
    type Target = str;
    fn deref(&self) -> &str {
        self.0.as_str()
    }
}

pub trait SubscriptionIds {
    fn subscription_ids(&self) -> Vec<SubscriptionId>;
}

impl SubscriptionIds for Vec<Subscription> {
    fn subscription_ids(&self) -> Vec<SubscriptionId> {
        self.iter()
            .map(|s| SubscriptionId(s.subscription_id.as_ref().unwrap().to_owned()))
            .collect::<Vec<SubscriptionId>>()
    }
}

impl fmt::Debug for AzureClient {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "AzureClient")
    }
}

impl AzureClient {
    #[instrument]
    pub async fn new(tenant_id: &str, client_id: &str, client_secret: &str) -> Result<Self> {
        let http_client = azure_core::new_http_client();
        let credentials = Arc::new(ClientSecretCredential::new(
            http_client,
            tenant_id.to_owned(),
            client_id.to_owned(),
            client_secret.to_owned(),
            azure_identity::TokenCredentialOptions::default(),
        ));
        Ok(AzureClient { credentials })
    }

    #[instrument]
    pub fn subscription_client(&self) -> azure_mgmt_subscription::Client {
        azure_mgmt_subscription::Client::new(
            "https://management.azure.com",
            self.credentials.clone(),
            vec!["https://management.azure.com".to_owned()],
            ClientOptions::default(),
        )
    }

    #[instrument]
    pub async fn subscriptions_list_send_events(
        &self,
        template_event: &Event,
        event_writer: Sender<Event>,
    ) -> Result<Vec<Subscription>> {
        let subscriptions = self
            .subscription_client()
            .subscriptions_client()
            .list()
            .into_stream()
            .map_ok(|s| futures::stream::iter(s.value))
            .flat_map(|s| s.unwrap())
            .inspect(|s| {
                let new_event = template_event.clone().data_from_ssphp_run(&s).unwrap();
                event_writer.send(new_event).unwrap()
            })
            .collect::<Vec<Subscription>>()
            .await;

        Ok(subscriptions)
    }

    #[instrument]
    pub fn resources_client(&self) -> azure_mgmt_resources::Client {
        azure_mgmt_resources::package_resources_2021_04::Client::new(
            "https://management.azure.com",
            self.credentials.clone(),
            vec!["https://management.azure.com".to_owned()],
            ClientOptions::default(),
        )
    }

    pub async fn resource_groups_list_send_events(
        &self,
        subscription_id: &SubscriptionId,
        template_event: &Event,
        event_writer: Sender<Event>,
    ) -> Result<()> {
        self.resources_client()
            .resource_groups_client()
            .list(subscription_id.deref().to_owned())
            .into_stream()
            .map_ok(|rg| futures::stream::iter(rg.value))
            .flat_map(|s| s.unwrap())
            .for_each_concurrent(None, |s| {
                let ew = event_writer.clone();
                let te = template_event.clone();
                async move {
                    let new_event = te.data_from_ssphp_run(&s).unwrap().source(&format!(
                        "azure_resource_group:subscription:{}",
                        &subscription_id.0
                    ));
                    ew.send(new_event).unwrap()
                }
            })
            .await;
        Ok(())
    }

    #[instrument]
    pub fn security_client(&self) -> azure_mgmt_security::package_composite_v3::Client {
        azure_mgmt_security::package_composite_v3::Client::new(
            "https://management.azure.com",
            self.credentials.clone(),
            vec!["https://management.azure.com".to_owned()],
            ClientOptions::default(),
        )
    }

    pub async fn alerts_list_send_events(
        &self,
        subscription_id: &SubscriptionId,
        template_event: &Event,
        event_writer: Sender<Event>,
    ) -> Result<()> {
        self.security_client()
            .alerts_client()
            .list(subscription_id.deref().to_owned())
            .into_stream()
            .map_ok(|a| futures::stream::iter(a.value))
            .flat_map(Result::unwrap)
            .for_each_concurrent(None, |s| {
                let ew = event_writer.clone();
                let te = template_event.clone();
                async move {
                    let new_event = te.data_from_ssphp_run(&s).unwrap().source(&format!(
                        "azure:security:alerts:subscription:{}",
                        &subscription_id.0
                    ));
                    ew.send(new_event).unwrap()
                }
            })
            .await;
        Ok(())
    }

    pub async fn secure_scores_list_send_events(
        &self,
        subscription_id: &SubscriptionId,
        template_event: &Event,
        event_writer: Sender<Event>,
    ) -> Result<()> {
        self.security_client()
            .secure_scores_client()
            .list(subscription_id.deref().to_owned())
            .into_stream()
            .map_ok(|a| futures::stream::iter(a.value))
            .flat_map(Result::unwrap)
            .for_each_concurrent(None, |s| {
                let ew = event_writer.clone();
                let te = template_event.clone();
                async move {
                    let new_event = te.data_from_ssphp_run(&s).unwrap().source(&format!(
                        "azure:security:score:subscription:{}",
                        &subscription_id.0
                    ));
                    ew.send(new_event).unwrap()
                }
            })
            .await;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::sync::mpsc::channel;
    use std::sync::mpsc::{Receiver, Sender};
    use std::time::SystemTime;

    async fn azure_client() -> AzureClient {
        AzureClient::new(
            &env::var("azure_tenant_id").expect("`azure_tenant_id` must be set"),
            &env::var("azure_client_id").expect("`azure_client_id` must be set"),
            &env::var("azure_client_secret").expect("`azure_client_secret` must be set"),
        )
        .await
        .expect("Failed to build AzureClient")
    }

    fn template_event() -> Event {
        let time = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("Failed to get time")
            .as_secs_f64();

        Event::new().time(time)
    }

    async fn subscription_ids(client: &AzureClient) -> Result<Vec<SubscriptionId>> {
        let template_event = template_event();
        let subscription_ids = {
            let (sender, _receiver) = channel::<Event>();
            client
                .subscriptions_list_send_events(&template_event, sender.clone())
                .await
                .expect("Faild while getting subscriptions")
                .subscription_ids()
        };
        Ok(subscription_ids)
    }

    async fn test_items() -> (AzureClient, Event, Sender<Event>, Receiver<Event>) {
        let (sender, receiver) = channel::<Event>();
        (azure_client().await, template_event(), sender, receiver)
    }

    #[tokio::test]
    async fn test_subscription_list_event_sender() {
        let (client, template_event, sender, receiver) = test_items().await;

        let subscriptions = client
            .subscriptions_list_send_events(&template_event, sender)
            .await
            .expect("Failed while getting subscriptions");

        let received_subscriptions = receiver.iter().collect::<Vec<Event>>();

        dbg!(&subscriptions, &received_subscriptions);

        assert_eq!(
            dbg!(&subscriptions.len()),
            dbg!(&received_subscriptions.len())
        );
    }

    #[tokio::test]
    async fn test_resource_group_list_event_sender_iter() {
        let (client, template_event, sender, receiver) = test_items().await;
        let subscription_ids = subscription_ids(&client).await.unwrap();

        futures::stream::iter(subscription_ids.iter())
            .for_each_concurrent(10, |sub_id| {
                let te = template_event.clone();
                let ew = sender.clone();
                let c = client.clone();
                async move {
                    c.resource_groups_list_send_events(sub_id, &te, ew)
                        .await
                        .expect("Failed while getting subscriptions");
                }
            })
            .await;

        std::mem::drop(sender);
        let received_resource_groups = receiver.iter().collect::<Vec<Event>>();
        dbg!(&received_resource_groups);
        assert!(received_resource_groups.len() > 0);
    }

    #[tokio::test]
    async fn test_alerts_list_event_sender_iter() {
        let (client, template_event, sender, receiver) = test_items().await;
        let subscription_ids = subscription_ids(&client).await.unwrap();

        futures::stream::iter(subscription_ids.iter())
            .for_each_concurrent(10, |sub_id| {
                let te = template_event.clone();
                let ew = sender.clone();
                let c = client.clone();
                async move {
                    c.alerts_list_send_events(sub_id, &te, ew)
                        .await
                        .expect("Failed while getting alerts");
                }
            })
            .await;

        std::mem::drop(sender);
        let received_resource_groups = receiver.iter().collect::<Vec<Event>>();
        dbg!(&received_resource_groups);
        assert!(received_resource_groups.len() > 0);
    }

    #[tokio::test]
    async fn test_secure_score_list_event_sender_iter() {
        let (client, template_event, sender, receiver) = test_items().await;
        let subscription_ids = subscription_ids(&client).await.unwrap();

        futures::stream::iter(subscription_ids.iter())
            .for_each_concurrent(10, |sub_id| {
                let te = template_event.clone();
                let ew = sender.clone();
                let c = client.clone();
                async move {
                    c.secure_scores_list_send_events(sub_id, &te, ew)
                        .await
                        .expect("Failed while getting Secure Score");
                }
            })
            .await;

        std::mem::drop(sender);
        let received_resource_groups = receiver.iter().collect::<Vec<Event>>();
        dbg!(&received_resource_groups);
        assert!(received_resource_groups.len() > 0);
    }
}
