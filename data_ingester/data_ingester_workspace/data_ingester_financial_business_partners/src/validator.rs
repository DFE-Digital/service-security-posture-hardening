use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use anyhow::{Context, Result};
use data_ingester_splunk_search::search_client::SplunkApiClient;
use data_ingester_supporting::keyvault::Secrets;
use serde::{Deserialize, Serialize};
use tracing::info;

/// Validate product, service_line, product tripplet against FBP data
#[derive(Debug, Deserialize, Default)]
//#[serde(transparent)]
pub struct Validator(HashMap<String, HashMap<String, HashSet<String>>>);

impl From<Vec<SsphpListFbpTaxonomy>> for Validator {
    fn from(value: Vec<SsphpListFbpTaxonomy>) -> Self {
        let mut validator = HashMap::new();
        for fbp in value.into_iter() {
            let _ = validator
                .entry(fbp.portfolio)
                .or_insert(HashMap::new())
                .entry(fbp.service_line)
                .or_insert(HashSet::from_iter(fbp.product.into_iter()));
        }
        Validator(validator)
    }
}

#[derive(Debug, Deserialize)]
/// Type recieved from Splunk Query
struct SsphpListFbpTaxonomy {
    portfolio: String,
    service_line: String,
    product: Vec<String>,
}

impl Validator {
    // Create a new validator
    pub async fn from_splunk_fbp(secrets: Arc<Secrets>) -> Result<Self> {
        let mut search_client = SplunkApiClient::new_from_secrets(secrets.clone())?.set_app("DCAP");

        search_client
            .open_acs()
            .await
            .context("Opening Splunk access via ACS")?;

        let search = "| savedsearch ssphp_list_fbp_taxonomy";

        info!("Running splunk search '{}'", search);

        let fbp_results = search_client
            .run_search::<SsphpListFbpTaxonomy>(search)
            .await
            .context("Running Splunk Search")?;

        search_client
            .close_acs()
            .await
            .context("Closing Splunk access via ACS")?;

        Ok(fbp_results.into())
    }

    #[allow(dead_code)]
    fn portfolio_exists(&self, portfolio: Option<&str>) -> bool {
        portfolio
            .map(|portfolio| self.0.contains_key(portfolio))
            .unwrap_or(false)
    }

    fn service_line_exists(&self, service_line: Option<&str>) -> bool {
        service_line
            .map(|service_line| {
                self.0
                    .values()
                    .any(|portfolio| portfolio.contains_key(service_line))
            })
            .unwrap_or(false)
    }

    fn product_exists(&self, product: Option<&str>) -> bool {
        product
            .map(|product| {
                self.0
                    .values()
                    .flat_map(|service_line| service_line.values())
                    .any(|service_line| service_line.contains(product))
            })
            .unwrap_or(false)
    }

    pub fn validate(
        &self,
        portfolio: Option<&str>,
        service_line: Option<&str>,
        product: Option<&str>,
    ) -> ValidationResult {
        let service_line_hashmap = portfolio.and_then(|portfolio| self.0.get(portfolio));

        let product_hashset = service_line.and_then(|service_line| {
            service_line_hashmap.and_then(|service_line_hashmap| service_line_hashmap.get(service_line))
        });

        let product_entry = product.and_then(|product| {
            product_hashset.and_then(|product_hashset| product_hashset.get(product))
        });

        ValidationResult {
            valid: product_entry.is_some(),
            portfolio_valid: service_line_hashmap.is_some(),
            service_line_valid: product_hashset.is_some(),
            product_valid: product_entry.is_some(),
            portfolio_exists: service_line_hashmap.is_some(),
            service_line_exists: product_hashset
                .map_or_else(|| self.service_line_exists(service_line), |_| true),
            product_exists: product_entry.map_or_else(|| self.product_exists(product), |_| true),
        }
    }
}

#[derive(PartialEq, Eq, Debug, Serialize)]
pub struct ValidationResult {
    /// Is the tripplet valid
    pub valid: bool,
    portfolio_valid: bool,
    portfolio_exists: bool,
    service_line_valid: bool,
    service_line_exists: bool,
    product_valid: bool,
    product_exists: bool,
}

#[cfg(test)]
mod test {
    use std::collections::{HashMap, HashSet};

    use super::{ValidationResult, Validator};

    fn build_validator() -> Validator {
        let mut products = HashSet::new();
        let _ = products.insert("product1".into());
        let mut service_lines = HashMap::new();
        let _ = service_lines.insert("service_line1".into(), products);
        let mut portfolios = HashMap::new();
        let _ = portfolios.insert("portfolio1".into(), service_lines);
        Validator(portfolios)
    }

    #[test]
    fn test_correct_tripplet() {
        let validator = build_validator();
        let result =
            validator.validate(Some("portfolio1"), Some("service_line1"), Some("product1"));
        let expected = ValidationResult {
            valid: true,
            portfolio_valid: true,
            portfolio_exists: true,
            service_line_valid: true,
            service_line_exists: true,
            product_valid: true,
            product_exists: true,
        };

        assert_eq!(result, expected);
    }

    #[test]
    fn test_missing_product() {
        let validator = build_validator();
        let result = validator.validate(Some("portfolio1"), Some("service_line1"), None);
        let expected = ValidationResult {
            valid: false,
            portfolio_valid: true,
            portfolio_exists: true,
            service_line_valid: true,
            service_line_exists: true,
            product_valid: false,
            product_exists: false,
        };

        assert_eq!(result, expected);
    }

    #[test]
    fn test_missing_service_line() {
        let validator = build_validator();
        let result = validator.validate(Some("portfolio1"), None, Some("product1"));
        let expected = ValidationResult {
            valid: false,
            portfolio_valid: true,
            portfolio_exists: true,
            service_line_valid: false,
            service_line_exists: false,
            product_valid: false,
            product_exists: true,
        };

        assert_eq!(result, expected);
    }

    #[test]
    fn test_missing_portfolio() {
        let validator = build_validator();
        let result = validator.validate(None, Some("service_line1"), Some("product1"));
        let expected = ValidationResult {
            valid: false,
            portfolio_valid: false,
            portfolio_exists: false,
            service_line_valid: false,
            service_line_exists: true,
            product_valid: false,
            product_exists: true,
        };

        assert_eq!(result, expected);
    }
}
