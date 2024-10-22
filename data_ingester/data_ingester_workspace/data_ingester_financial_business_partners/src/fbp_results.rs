use std::{collections::HashSet, sync::Arc};

use anyhow::{Context, Result};
use data_ingester_splunk_search::search_client::SplunkApiClient;
use data_ingester_supporting::keyvault::Secrets;
use serde::Deserialize;
use tracing::info;

#[derive(Debug, Deserialize)]
pub struct FbpResult {
    pub portfolio: String,
    pub service_line: String,
    pub product: Vec<String>,
}

#[derive(Debug)]
pub struct FbpResults {
    pub results: Vec<FbpResult>,
}

impl From<Vec<FbpResult>> for FbpResults {
    fn from(value: Vec<FbpResult>) -> Self {
        Self { results: value }
    }
}

impl FbpResults {
    pub async fn get_results_from_splunk(secrets: Arc<Secrets>) -> Result<Self> {
        let mut search_client =
            SplunkApiClient::new_from_secrets(secrets.clone())?.set_app("DCAP_DEV");

        search_client
            .open_acs()
            .await
            .context("Opening Splunk access via ACS")?;

        let search = "| savedsearch ssphp_list_fbp_taxonomy_DEV";

        info!("Running splunk search '{}'", search);
        let fbp_results = search_client
            .run_search::<FbpResult>(search)
            .await
            .context("Running Splunk Search")?;

        // search_client
        //     .close_acs()
        //     .await
        //     .context("Closing Splunk access via ACS")?;
        Ok(fbp_results.into())
    }

    pub fn portfolios(&self) -> Vec<&str> {
        self.results
            .iter()
            .map(|result| result.portfolio.as_str())
            .collect::<HashSet<&str>>()
            .into_iter()
            .collect()
    }

    pub fn service_lines(&self) -> Vec<&str> {
        self.results
            .iter()
            .map(|result| result.service_line.as_str())
            .collect::<HashSet<&str>>()
            .into_iter()
            .collect()
    }
}

#[cfg(test)]
mod test {
    use super::{FbpResult, FbpResults};

    fn fbp_results() -> FbpResults {
        FbpResults {
            results: vec![
                FbpResult {
                    portfolio: "po1".into(),
                    service_line: "sl1".into(),
                    product: vec!["pr1-1".into(), "pr1-2".into()],
                },
                FbpResult {
                    portfolio: "po2".into(),
                    service_line: "sl2".into(),
                    product: vec!["pr2-1".into(), "pr2-2".into()],
                },
                FbpResult {
                    portfolio: "po3".into(),
                    service_line: "sl3".into(),
                    product: vec!["pr3-1".into(), "pr3-2".into()],
                },
            ],
        }
    }

    #[test]
    fn test_fbp_results_portfolios() {
        let fbp_results = fbp_results();
        let portfolios = fbp_results.portfolios();
        assert_eq!(portfolios.len(), 3);
        assert!(portfolios.iter().all(|p| p.starts_with("po")));
    }

    #[test]
    fn test_fbp_results_service_lines() {
        let fbp_results = fbp_results();
        let service_lines = fbp_results.service_lines();
        assert_eq!(service_lines.len(), 3);
        assert!(service_lines.iter().all(|p| p.starts_with("sl")));
    }
}

#[cfg(feature = "live_tests")]
#[cfg(test)]
mod live_tests {
    use std::{env, sync::Arc};

    use super::FbpResults;
    use data_ingester_splunk::splunk::set_ssphp_run;
    use data_ingester_supporting::keyvault::get_keyvault_secrets;

    #[tokio::test]
    async fn test_fbp_results_get_results_from_splunk() -> anyhow::Result<()> {
        let secrets = get_keyvault_secrets(
            &env::var("KEY_VAULT_NAME").expect("Need KEY_VAULT_NAME enviornment variable"),
        )
        .await
        .unwrap();
        set_ssphp_run("fbp")?;

        let fbp = FbpResults::get_results_from_splunk(Arc::new(secrets)).await?;

        assert!(!fbp.results.is_empty());

        Ok(())
    }
}
