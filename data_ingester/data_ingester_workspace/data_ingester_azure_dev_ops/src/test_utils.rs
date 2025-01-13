use std::sync::LazyLock;

use crate::ado_dev_ops_client::AzureDevOpsClient;
use anyhow::Context;
use data_ingester_splunk::splunk::Splunk;
use data_ingester_supporting::keyvault::get_keyvault_secrets;
use tracing::subscriber::DefaultGuard;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt};

pub(crate) struct TestSetup {
    pub(crate) ado: AzureDevOpsClient,
    pub(crate) organization: String,
    pub(crate) splunks: Vec<Splunk>,
    #[allow(unused)]
    pub(crate) tracing_guard: DefaultGuard,
    pub(crate) runtime: tokio::runtime::Runtime,
}

pub(crate) static TEST_SETUP: LazyLock<TestSetup> = LazyLock::new(test_setup_setup);

fn test_setup_setup() -> TestSetup {
    let runtime = tokio::runtime::Runtime::new().unwrap();

    let (ado, organization, splunks, tracing_guard) = runtime.block_on(async {
        let subscriber =
            tracing_subscriber::FmtSubscriber::new().with(EnvFilter::from_default_env());
        let tracing_guard = tracing::subscriber::set_default(subscriber);

        let secrets = get_keyvault_secrets(
            &std::env::var("KEY_VAULT_NAME").expect("Need KEY_VAULT_NAME enviornment variable"),
        )
        .await
        .unwrap();
        let ado = AzureDevOpsClient::new(
            secrets
                .azure_client_id
                .as_ref()
                .context("No Azure Client Id")
                .unwrap(),
            secrets
                .azure_client_secret
                .as_ref()
                .context("No Azure Client Secret")
                .unwrap(),
            secrets
                .azure_tenant_id
                .as_ref()
                .context("No Azure Tenant Id")
                .unwrap(),
        )
        .await
        .unwrap();

        let splunk = Splunk::new(
            secrets.splunk_host.as_ref().context("No value").unwrap(),
            secrets.splunk_token.as_ref().context("No value").unwrap(),
            false,
        )
        .unwrap();

        let splunk_ian = Splunk::new(
            secrets
                .ian_splunk_host
                .as_ref()
                .context("No value")
                .unwrap(),
            secrets
                .ian_splunk_token
                .as_ref()
                .context("No value")
                .unwrap(),
            true,
        )
        .unwrap();

        let organization = "aktest0831";
        (
            ado,
            organization.to_string(),
            vec![splunk, splunk_ian],
            tracing_guard,
        )
    });
    TestSetup {
        ado,
        organization,
        splunks,
        tracing_guard,
        runtime,
    }
}
