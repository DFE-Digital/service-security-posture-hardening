#[cfg(feature = "live_tests")]
use crate::azure_dev_ops_client_oauth::AzureDevOpsClientOauth;
use crate::{
    ado_dev_ops_client::{AzureDevOpsClient, AzureDevOpsClientMethods},
    data::{projects::Project, repositories::Repository},
};
#[cfg(feature = "live_tests")]
use anyhow::Context;
use anyhow::Result;
use data_ingester_splunk::splunk::{HecEvent, SplunkTrait};
#[cfg(feature = "live_tests")]
use data_ingester_supporting::keyvault::get_keyvault_secrets;
use std::sync::LazyLock;
use tokio::sync::mpsc::Receiver;
use tokio::sync::mpsc::{channel, Sender};
use tracing_subscriber::EnvFilter;

pub struct SplunkTester {
    send_tx: Sender<HecEvent>,
    send_rx: Receiver<HecEvent>,
}

impl SplunkTester {
    fn new() -> Result<Self> {
        let (send_tx, send_rx) = channel::<HecEvent>(5000);
        Ok(Self { send_tx, send_rx })
    }

    #[allow(dead_code)]
    async fn recv_events(&mut self) -> Vec<HecEvent> {
        let mut buffer = vec![];
        let _received_events_count = self.send_rx.recv_many(&mut buffer, 5001).await;
        buffer
    }
}

impl SplunkTrait for SplunkTester {
    fn new(_host: &str, _token: &str, _hec_acknowledgment: bool) -> anyhow::Result<Self> {
        unimplemented!()
    }

    fn new_request_client(
        _token: &str,
        _hec_acknowledgment: bool,
    ) -> anyhow::Result<reqwest::Client> {
        unimplemented!()
    }

    fn headers(
        _token: &str,
        _hec_acknowledgment: bool,
    ) -> anyhow::Result<reqwest::header::HeaderMap> {
        unimplemented!()
    }

    fn send_tx(&self) -> &tokio::sync::mpsc::Sender<HecEvent> {
        &self.send_tx
    }
}

#[allow(unused)]
pub(crate) struct TestSetup<A: AzureDevOpsClientMethods, S: SplunkTrait> {
    pub(crate) ado: A,
    pub(crate) organization: String,
    pub(crate) project: Project,
    pub(crate) repo: Repository,
    pub(crate) splunk: S,
    #[allow(unused)]
    // pub(crate) tracing_guard: DefaultGuard,
    pub(crate) runtime: tokio::runtime::Runtime,
}

pub(crate) struct AzureDevOpsTestClient {
    #[cfg(feature = "live_tests")]
    real_client: AzureDevOpsClientOauth,
}

impl AzureDevOpsClient for AzureDevOpsTestClient {
    async fn get<
        T: serde::de::DeserializeOwned + crate::ado_response::AddAdoResponse + serde::Serialize,
    >(
        &self,
        metadata: crate::ado_metadata::AdoMetadata,
    ) -> anyhow::Result<crate::ado_response::AdoResponse> {
        let url = metadata.url().replace("https://", "").replace("/", "_");

        let storage_key = format!("test/{}.json", url);

        let response = match std::fs::read_to_string(&storage_key) {
            Ok(test_json) => test_json,

            #[cfg(not(feature = "live_tests"))]
            Err(err) => {
                // #[cfg(feature = "live_tests")]
                // let test_json = {
                //     // Get real data from the API
                //     let ado_response = self
                //         .real_client
                //         .get::<T>(metadata)
                //         .await
                //         .expect("request to complete");

                //     // Save the data to the test fixtures
                //     let json = serde_json::to_string(&ado_response)
                //         .expect("ado_response should serialize");
                //     std::fs::write(&storage_key, json.as_bytes()).expect("file should be written");

                //     json
                // };

                let error = format!("Unable to read file {storage_key}: {}", &err);
                panic!("{}", error);
            }

            #[cfg(feature = "live_tests")]
            Err(_err) => {
                let test_json = {
                    // Get real data from the API
                    let ado_response = self
                        .real_client
                        .get::<T>(metadata)
                        .await
                        .expect("request to complete");

                    // Save the data to the test fixtures
                    let json = serde_json::to_string(&ado_response)
                        .expect("ado_response should serialize");
                    std::fs::write(&storage_key, json.as_bytes()).expect("file should be written");

                    json
                };

                test_json
            }
        };

        let response = serde_json::from_str(&response).expect("Test data should parse");

        Ok(response)
    }
}

impl AzureDevOpsClientMethods for AzureDevOpsTestClient {}

#[allow(unused)]
pub(crate) static TEST_SETUP: LazyLock<TestSetup<AzureDevOpsTestClient, SplunkTester>> =
    LazyLock::new(test_setup_setup);

#[cfg(test)]
fn test_setup_setup() -> TestSetup<AzureDevOpsTestClient, SplunkTester> {
    use crate::{data::projects::Project, data::repositories::Repository};

    let runtime = tokio::runtime::Runtime::new().unwrap();

    tracing_subscriber::FmtSubscriber::builder()
        .with_env_filter(EnvFilter::from_env("RUST_LOG"))
        .init();

    let (ado, splunk) = runtime.block_on(async {
        #[cfg(feature = "live_tests")]
        let secrets = get_keyvault_secrets(
            &std::env::var("KEY_VAULT_NAME").expect("Need KEY_VAULT_NAME enviornment variable"),
        )
        .await
        .unwrap();

        #[cfg(feature = "live_tests")]
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

        let ado = AzureDevOpsTestClient {
            #[cfg(feature = "live_tests")]
            real_client: ado,
        };

        let splunk = SplunkTester::new().unwrap();

        (ado, splunk)
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
        splunk,
        runtime,
    }
}
