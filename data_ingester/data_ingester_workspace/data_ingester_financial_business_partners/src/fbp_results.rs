use anyhow::{Context, Result};
use data_ingester_splunk_search::search_client::SplunkApiClient;
use data_ingester_supporting::keyvault::Secrets;
use serde::Deserialize;
use std::sync::Arc;
use tracing::info;

#[derive(Debug, Deserialize, Default)]
pub struct FbpResult {
    #[serde(rename(deserialize = "portfolio"))]
    pub portfolios: Vec<String>,
    #[serde(rename(deserialize = "service_line"))]
    pub service_lines: Vec<String>,
    #[serde(rename(deserialize = "product"))]
    pub products: Vec<String>,
}

impl From<Vec<FbpResult>> for FbpResult {
    fn from(value: Vec<FbpResult>) -> Self {
        value
            .into_iter()
            .fold(FbpResult::default(), |mut acc, value| {
                acc.portfolios.extend(value.portfolios);
                acc.service_lines.extend(value.service_lines);
                acc.products.extend(value.products);
                acc
            })
    }
}

impl FbpResult {
    pub async fn get_results_from_splunk(secrets: Arc<Secrets>) -> Result<Self> {
        let mut search_client = SplunkApiClient::new_from_secrets(secrets.clone())?.set_app("DCAP");

        search_client
            .open_acs()
            .await
            .context("Opening Splunk access via ACS")?;

        let search = "| savedsearch ssphp_list_fbp_taxonomy_github_custom_properties";

        info!("Running splunk search '{}'", search);

        let fbp_results = search_client
            .run_search::<FbpResult>(search)
            .await
            .context("Running Splunk Search")?;

        search_client
            .close_acs()
            .await
            .context("Closing Splunk access via ACS")?;

        Ok(fbp_results.into())
    }

    pub fn portfolios(&self) -> &[String] {
        self.portfolios.as_slice()
    }

    pub fn service_lines(&self) -> &[String] {
        self.service_lines.as_slice()
    }

    pub fn products(&self) -> &[String] {
        self.service_lines.as_slice()
    }

    pub fn is_empty(&self) -> bool {
        self.portfolios.is_empty() || self.service_lines.is_empty() || self.products.is_empty()
    }
}

#[cfg(test)]
mod test {
    use super::FbpResult;

    fn fbp_results() -> FbpResult {
        FbpResult {
            portfolios: vec!["po1".into(), "po2".into(), "po3".into()],
            service_lines: vec!["sl1".into(), "sl2".into(), "sl3".into()],
            products: vec!["pr1-1".into(), "pr1-2".into()],
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

    use super::FbpResult;
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

        let fbp = FbpResult::get_results_from_splunk(Arc::new(secrets)).await?;

        assert!(!fbp.portfolios().is_empty());
        assert!(!fbp.service_lines().is_empty());
        assert!(!fbp.products().is_empty());

        Ok(())
    }
}
