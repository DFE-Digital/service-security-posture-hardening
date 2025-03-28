use std::sync::LazyLock;

use crate::{
    azure_dev_ops_client_oauth::AzureDevOpsClientOauth,
    data::{projects::Project, repositories::Repository},
};
use anyhow::Context;
use data_ingester_splunk::splunk::Splunk;
use data_ingester_supporting::keyvault::get_keyvault_secrets;
use tracing_subscriber::EnvFilter;

#[allow(unused)]
pub(crate) struct TestSetup {
    pub(crate) ado: AzureDevOpsClientOauth,
    pub(crate) organization: String,
    pub(crate) project: Project,
    pub(crate) repo: Repository,
    pub(crate) splunks: Vec<Splunk>,
    #[allow(unused)]
    // pub(crate) tracing_guard: DefaultGuard,
    pub(crate) runtime: tokio::runtime::Runtime,
}

#[allow(unused)]
pub(crate) static TEST_SETUP: LazyLock<TestSetup> = LazyLock::new(test_setup_setup);

#[cfg(test)]
fn test_setup_setup() -> TestSetup {
    use crate::{
        azure_dev_ops_client_oauth::AzureDevOpsClientOauth,
        data::{projects::Project, repositories::Repository},
    };

    let runtime = tokio::runtime::Runtime::new().unwrap();

    tracing_subscriber::FmtSubscriber::builder()
        .with_env_filter(EnvFilter::from_env("RUST_LOG"))
        .init();

    let (ado, splunks) = runtime.block_on(async {
        let secrets = get_keyvault_secrets(
            &std::env::var("KEY_VAULT_NAME").expect("Need KEY_VAULT_NAME enviornment variable"),
        )
        .await
        .unwrap();
        let ado = AzureDevOpsClientOauth::new(
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

        (ado, vec![splunk, splunk_ian])
    });

    let organization = "aktest0831".to_string();
    let project = Project {
        name: "foo".into(),
        id: "foo".into(),
        ..Default::default()
    };
    let repo = Repository::new("bar".into(), "bar".into());

    TestSetup {
        ado,
        organization,
        project,
        repo,
        splunks,
        runtime,
    }
}
