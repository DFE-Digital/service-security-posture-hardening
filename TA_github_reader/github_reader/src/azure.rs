use anyhow::Result;
use azure_core::ClientOptions;
use azure_identity::ClientSecretCredential;
// use azure_mgmt_security::models::{SecurityAssessmentList, SecurityTaskList};
// use azure_mgmt_security::package_composite_v3::Client as SecurityClient;
use azure_mgmt_subscription::models::{Subscription, SubscriptionListResult};
use futures::StreamExt;
use modular_input::Event;

use std::sync::mpsc::Sender;
use std::sync::Arc;

use std::fmt;
use tracing::instrument;

pub struct AzureClient {
    credentials: Arc<ClientSecretCredential>,
}

impl fmt::Debug for AzureClient {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "AzureClient")
    }
}

impl AzureClient {
    #[instrument]
    pub async fn new(tenant_id: &str, client_id: &str, client_secret: &str) -> Result<Self> {
        //let credentials = Arc::new(AzureCliCredential::new());
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
    pub fn subscription_client(&self) -> Result<azure_mgmt_subscription::Client> {
        let subscription_client = azure_mgmt_subscription::Client::new(
            "https://management.azure.com",
            self.credentials.clone(),
            vec!["https://management.azure.com".to_owned()],
            ClientOptions::default(),
        );
        Ok(subscription_client)
    }

    pub async fn subscriptions_list(&mut self) -> Result<Vec<Subscription>> {
        let subscriptions = self
            .subscription_client()?
            .subscriptions_client()
            .list()
            .into_stream()
            .map(|c| c.unwrap())
            .collect::<Vec<SubscriptionListResult>>()
            .await
            .into_iter()
            .flat_map(|s| s.value)
            .collect::<Vec<Subscription>>();

        Ok(subscriptions)
    }

    #[instrument]
    pub async fn subscriptions_list_send_events(
        &self,
        template_event: &Event,
        event_writer: Sender<Event>,
    ) -> Result<Vec<Subscription>> {
        let mut subs = vec![];
        self.subscription_client()?
            .subscriptions_client()
            .list()
            .into_stream()
            .map(|c| c.unwrap())
            .collect::<Vec<SubscriptionListResult>>()
            .await
            .into_iter()
            .flat_map(|s| s.value)
            .inspect(|s| subs.push(s.clone()))
            .map(|s| template_event.clone().data_from_ssphp_run(&s).unwrap())
            .for_each(|s| {
                event_writer.send(s).unwrap();
            });

        Ok(subs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::sync::mpsc::channel;
    use std::time::SystemTime;

    async fn test_azure_client() -> AzureClient {
        AzureClient::new(
            &env::var("azure_tenant_id").unwrap(),
            &env::var("azure_client_id").unwrap(),
            &env::var("azure_client_secret").unwrap(),
        )
        .await
        .unwrap()
    }

    #[tokio::test]
    async fn test_subscripition_list() {
        let mut client = test_azure_client().await;
        let subscriptions = client.subscriptions_list().await.unwrap();
        dbg!(&subscriptions);
        assert!(subscriptions.len() > 0);
    }

    #[tokio::test]
    async fn test_subscription_list_event_sender() {
        let time = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs_f64();

        let template_event = Event::new()
            .source("Azure")
            .sourcetype("azure:subscription")
            .time(time);
        let (sender, receiver) = channel::<Event>();

        let client = test_azure_client().await;
        let subscriptions = client
            .subscriptions_list_send_events(&template_event, sender)
            .await
            .unwrap();
        let received_subscriptions = receiver.iter().collect::<Vec<Event>>();
        dbg!(&subscriptions, &received_subscriptions);
        assert_eq!(
            dbg!(&subscriptions.len()),
            dbg!(&received_subscriptions.len())
        );
    }
}
