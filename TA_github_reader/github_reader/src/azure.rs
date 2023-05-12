use anyhow::{Context, Result};
use futures::join;
use futures::StreamExt;
use modular_input::{Event, Input, ModularInput, Scheme};
use splunk_rest_client::Client as SplunkClient;
use std::sync::mpsc::Sender;

use crate::azure_client::{AzureClient, SubscriptionId, SubscriptionIds};

#[derive(Debug, Default)]
pub struct AzureMI {
    pub client_id: String,
    pub client_secret: String,
    pub tenant_id: String,
}

impl ModularInput for AzureMI {
    async fn from_input(input: &Input) -> Result<Self> {
        let client = SplunkClient::new(&input.server_uri, &input.session_key, false)?;
        // TODO figure out multiple creds  / tenants
        let tenant_id = client
            .get_password(crate::SPLUNK_APP_NAME, "tenant_id", None)
            .await?;
        let client_id = client
            .get_password(crate::SPLUNK_APP_NAME, "client_id", None)
            .await?;
        let client_secret = client
            .get_password(crate::SPLUNK_APP_NAME, "client_secret", None)
            .await?;
        Ok(Self {
            client_id,
            client_secret,
            tenant_id,
        })
    }

    async fn run(&self) -> Result<()> {
        let time = self.current_time()?;
        let (event_writer_thread, event_writer) = self.start_event_writing_thread();

        let client =
            AzureClient::new(&self.tenant_id, &self.client_id, &self.client_secret).await?;

        let azure = Azure { concurrency: 10 };

        let subscription_ids = azure
            .subscriptions(&client, event_writer.clone(), time)
            .await?;

        let resource_groups =
            azure.resource_groups(&client, &subscription_ids, event_writer.clone(), time);

        let alerts = azure.alerts(&client, &subscription_ids, event_writer.clone(), time);

        let secure_scores =
            azure.secure_scores(&client, &subscription_ids, event_writer.clone(), time);

        let (resource_groups, alerts, secure_scores) =
            join!(resource_groups, alerts, secure_scores);

        resource_groups.context("Failed to get ResourceGroups")?;
        alerts.context("Failed to get Alerts")?;
        secure_scores.context("Failed to get Secure Scores")?;

        std::mem::drop(event_writer);
        event_writer_thread.join().unwrap()?;
        Ok(())
    }

    async fn validate_arguments(&self) -> Result<()> {
        AzureClient::new(&self.tenant_id, &self.client_id, &self.client_secret).await?;
        Ok(())
    }

    fn scheme() -> Result<()> {
        let scheme = Scheme {
            title: "SSPHP_Azure".to_string(),
            description: "SSPHP Azure Client - Subscriptions, resource groups, security findings, security alerts ".to_string(),
            streaming_mode: "xml".to_string(),
            use_single_instance: false,
        };
        <Self as ModularInput>::write_event_xml(&scheme).map_err(anyhow::Error::from)?;
        Ok(())
    }
}

struct Azure {
    concurrency: usize,
}

impl Azure {
    async fn subscriptions(
        &self,
        client: &AzureClient,
        event_writer: Sender<Event>,
        time: f64,
    ) -> Result<Vec<SubscriptionId>> {
        let template_event = Event::new()
            .source("azure")
            .sourcetype("azure:subscription")
            .time(time);
        let subscriptions = client
            .subscriptions_list_send_events(&template_event, event_writer.clone())
            .await?;

        Ok(subscriptions.subscription_ids())
    }

    async fn resource_groups(
        &self,
        client: &AzureClient,
        subscription_ids: &[SubscriptionId],
        event_writer: Sender<Event>,
        time: f64,
    ) -> Result<()> {
        let template_event = Event::new().sourcetype("azure:resource:group").time(time);
        futures::stream::iter(subscription_ids.iter())
            .for_each_concurrent(self.concurrency, |sub_id| {
                let te = template_event.clone();
                let ew = event_writer.clone();
                let c = client.clone();
                async move {
                    c.resource_groups_list_send_events(sub_id, &te, ew)
                        .await
                        .expect("Faild while getting ResourceGroups");
                }
            })
            .await;
        Ok(())
    }

    async fn alerts(
        &self,
        client: &AzureClient,
        subscription_ids: &[SubscriptionId],
        event_writer: Sender<Event>,
        time: f64,
    ) -> Result<()> {
        let template_event = Event::new().sourcetype("azure:security:alert").time(time);
        futures::stream::iter(subscription_ids.iter())
            .for_each_concurrent(self.concurrency, |sub_id| {
                let te = template_event.clone();
                let ew = event_writer.clone();
                let c = client.clone();
                async move {
                    c.alerts_list_send_events(sub_id, &te, ew)
                        .await
                        .expect("Failed while getting Alerts");
                }
            })
            .await;
        Ok(())
    }

    async fn secure_scores(
        &self,
        client: &AzureClient,
        subscription_ids: &[SubscriptionId],
        event_writer: Sender<Event>,
        time: f64,
    ) -> Result<()> {
        let template_event = Event::new().sourcetype("azure:security:score").time(time);
        futures::stream::iter(subscription_ids.iter())
            .for_each_concurrent(self.concurrency, |sub_id| {
                let te = template_event.clone();
                let ew = event_writer.clone();
                let c = client.clone();
                async move {
                    c.secure_scores_list_send_events(sub_id, &te, ew)
                        .await
                        .expect("Failed while getting Secure Scores");
                }
            })
            .await;
        Ok(())
    }
}
