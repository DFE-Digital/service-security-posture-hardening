use std::sync::Arc;

use crate::github_response::{GithubResponse, GithubResponses};
use data_ingester_financial_business_partners::validator::{ValidationResult, Validator};
use data_ingester_splunk::splunk::ToHecEvents;
use itertools::{Either, Itertools};
use serde::{Deserialize, Serialize};
use tracing::error;

/// https://docs.github.com/en/rest/orgs/custom-properties?apiVersion=2022-11-28#create-or-update-custom-properties-for-an-organization
#[derive(Debug, Deserialize, Serialize)]
pub struct CustomPropertySetter {
    // The name of the property
    property_name: String,

    // The type of the value for the property
    // Can be one of: string, single_select, multi_select, true_false
    value_type: ValueType,

    // Whether the property is required.
    required: bool,

    // Default value of the property
    default_value: Option<DefaultValue>,

    // Short description of the property
    description: Option<String>,

    // An ordered list of the allowed values of the property. The property can have up to 200 allowed values.
    allowed_values: Option<Vec<String>>,

    // Who can edit the values of the property
    values_editable_by: Option<ValuesEditableBy>,
}

static SERVICE_LINE_CLEANER_DATA: [(&str, &str); 2] = [
    (
        "Northern Territorial\u{a0}Team & ESF",
        // Non breaking space^
        "Northen Territorial Team & ESF",
    ),
    (
        "Digital Delivery – OIG (Protected)",
        // EM Dash        ^
        "Digital Delivery - OIG (Protected)",
    ),
];

struct ServiceLineCleaner();

impl ServiceLineCleaner {
    pub(crate) fn new() -> Self {
        Self()
    }

    fn allowed_values_cleaner_to_github<'value, S: AsRef<str>>(
        &self,
        value: &'value S,
    ) -> &'value str {
        for (fbp, github) in SERVICE_LINE_CLEANER_DATA {
            if value.as_ref() == fbp {
                return github;
            }
        }
        value.as_ref()
    }

    fn allowed_values_cleaner_from_github<'value, S: AsRef<str>>(
        &self,
        value: &'value S,
    ) -> &'value str {
        for (fbp, github) in SERVICE_LINE_CLEANER_DATA {
            if value.as_ref() == github {
                return fbp;
            }
        }
        value.as_ref()
    }
}

impl CustomPropertySetter {
    pub fn new<N: Into<String>, D: Into<String>>(
        property_name: N,
        description: Option<D>,
        required: bool,
        value_type: ValueType,
    ) -> Self {
        Self {
            property_name: property_name.into(),
            //url: None,
            value_type,
            required,
            default_value: None,
            description: description.map(|d| d.into()),
            allowed_values: None,
            values_editable_by: None,
        }
    }

    pub fn new_single_select<
        V: AsRef<[S]>,
        S: AsRef<str> + std::fmt::Debug,
        N: Into<String>,
        D: Into<String>,
    >(
        property_name: N,
        description: Option<D>,
        required: bool,
        allowed_values: V,
    ) -> Self {
        Self {
            property_name: property_name.into(),
            value_type: ValueType::SingleSelect,
            required,
            default_value: None,
            description: description.map(|d| d.into()),
            allowed_values: Some(
                allowed_values
                    .as_ref()
                    .iter()
                    .map(|s| s.as_ref().to_string())
                    .collect(),
            ),
            values_editable_by: Some(ValuesEditableBy::OrgAndRepoActors),
        }
    }

    pub fn from_fbp_portfolio<V: AsRef<[S]>, S: AsRef<str> + std::fmt::Debug>(
        allowed_values: V,
    ) -> Self {
        CustomPropertySetter::new_single_select(
            "portfolio",
            Some("The portfolio"),
            false,
            allowed_values,
        )
    }

    pub fn from_fbp_service_line<V: AsRef<[S]>, S: AsRef<str> + std::fmt::Debug>(
        allowed_values: V,
    ) -> Self {
        let service_line_cleaner = ServiceLineCleaner::new();
        let (allowed_value_strings, invalid_service_lines): (Vec<String>, Vec<String>) =
            allowed_values
                .as_ref()
                .iter()
                .map(|value| {
                    service_line_cleaner
                        .allowed_values_cleaner_to_github(&value.as_ref())
                        .to_string()
                })
                .partition_map(|v| {
                    if v.is_ascii() {
                        Either::Left(v)
                    } else {
                        Either::Right(v)
                    }
                });

        invalid_service_lines.iter().for_each(|sl|{
            error!(name="github", failed_value=sl, "Failure while converting 'FBP Service Line' to GitHub CustomProperty allowed_value: {:?}", sl);});

        CustomPropertySetter::new_single_select(
            "service_line",
            Some("The Service Line"),
            false,
            allowed_value_strings,
        )
    }

    pub fn from_fbp_product() -> Self {
        let mut product_setter =
            CustomPropertySetter::new("product", Some("Product"), false, ValueType::String);
        product_setter.values_editable_by = Some(ValuesEditableBy::OrgAndRepoActors);
        product_setter
    }

    pub fn property_name(&self) -> &str {
        &self.property_name
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ValueType {
    String,
    SingleSelect,
    MultiSelect,
    TrueFalse,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub(crate) enum DefaultValue {
    Array(Vec<String>),
    String(String),
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ValuesEditableBy {
    OrgActors,
    OrgAndRepoActors,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) struct CustomProperties {
    pub(crate) custom_properties: Vec<CustomProperty>,
    source: String,
}

impl ToHecEvents for &CustomProperties {
    type Item = CustomProperty;

    fn source(&self) -> &str {
        self.source.as_str()
    }

    fn sourcetype(&self) -> &str {
        "github"
    }

    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        Box::new(self.custom_properties.iter())
    }

    fn ssphp_run_key(&self) -> &str {
        "github"
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) struct CustomProperty {
    repository_id: u64,
    repository_name: String,
    repository_full_name: String,
    properties: Vec<Property>,
    #[serde(skip_deserializing)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) validation_errors: Option<ValidationResult>,
}

impl CustomProperty {
    fn clean(&mut self) {
        let service_line_cleaner = ServiceLineCleaner::new();
        self.properties
            .iter_mut()
            .for_each(|property| match property.value {
                Some(VecOrString::VecString(ref mut vec)) => {
                    vec.iter_mut().for_each(|value| {
                        *value = service_line_cleaner
                            .allowed_values_cleaner_from_github(&value)
                            .to_string()
                    });
                }
                Some(VecOrString::String(ref mut value)) => {
                    *value = service_line_cleaner
                        .allowed_values_cleaner_from_github(&value)
                        .to_string();
                }
                _ => {}
            });
    }

    #[allow(dead_code)]
    pub(crate) fn portfolio(&self) -> Option<&str> {
        self.get_property_value("portfolio")
    }

    pub(crate) fn service_line(&self) -> Option<&str> {
        self.get_property_value("service_line")
    }

    pub(crate) fn product(&self) -> Option<&str> {
        self.get_property_value("product")
    }

    fn get_property_value(&self, key: &str) -> Option<&str> {
        let product = self
            .properties
            .iter()
            .find(|property| property.property_name == key);
        product
            .and_then(|property| property.value.as_ref())
            .map(|value| match value {
                VecOrString::VecString(_vec) => unreachable!("No '{}' should be a multivalue", key),
                VecOrString::String(ref inner) => inner.as_str(),
            })
    }

    pub(crate) fn validate(&mut self, validator: &Arc<Validator>) {
        let results = validator.validate(self.portfolio(), self.service_line(), self.product());
        if !results.valid {
            self.validation_errors = Some(results);
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) struct Property {
    property_name: String,
    value: Option<VecOrString>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
enum VecOrString {
    VecString(Vec<String>),
    String(String),
}

impl From<GithubResponses> for CustomProperties {
    fn from(value: GithubResponses) -> Self {
        let custom_properties = value
            .responses_iter()
            .map(CustomProperties::from)
            .flat_map(|custom_properties| custom_properties.custom_properties.into_iter())
            .collect();
        Self {
            custom_properties,
            source: value.source(),
        }
    }
}

impl From<&GithubResponse> for CustomProperties {
    fn from(value: &GithubResponse) -> Self {
        let (mut custom_properties, failures): (Vec<_>, Vec<_>) = value
            .into_iter()
            .map(|response| serde_json::from_value::<CustomProperty>(response.clone()))
            .partition_map(|r| match r {
                Ok(v) => Either::Left(v),
                Err(v) => Either::Right(v),
            });

        failures.iter().for_each(|failure| {
            error!(name="github", error=?failure, "Failure while converting GitHubResponse to CustomProperty");
        });

        custom_properties.iter_mut().for_each(|custom_property| {
            custom_property.clean();
        });

        Self {
            custom_properties,
            source: value.source().to_string(),
        }
    }
}

#[cfg(test)]
mod test {
    use data_ingester_financial_business_partners::fbp_results::FbpResult;

    use crate::custom_properties::CustomPropertySetter;

    fn fbp_results() -> FbpResult {
        FbpResult {
            portfolios: vec!["po1".into(), "po2".into(), "po3".into()],
            service_lines: vec!["sl1".into(), "sl2".into(), "sl3".into()],
            products: vec!["pr1-1".into(), "pr1-2".into()],
        }
    }

    #[test]
    fn test_portfolio_setter() {
        let fbp = fbp_results();

        let portfolio_setter = CustomPropertySetter::from_fbp_portfolio(fbp.portfolios());
        let mut json = serde_json::to_value(&portfolio_setter).unwrap();

        let mut expected_json = serde_json::to_value(serde_json::json!({
            "property_name": "portfolio",
            "value_type": "single_select",
            "required": false,
            "default_value": null,
            "description": "The portfolio",
            "allowed_values": fbp.portfolios(),
            "values_editable_by": "org_and_repo_actors",
        }))
        .unwrap();

        // Check all allowed_values exist in the output object
        fbp.portfolios()
            .iter()
            .flat_map(serde_json::to_value)
            .for_each(|po| {
                assert!(json
                    .get("allowed_values")
                    .expect("allowed_values should have items")
                    .as_array()
                    .expect("should be array")
                    .contains(&po))
            });

        // Remove allowed values due to array ordering Eq
        let _ = json.get_mut("allowed_values").unwrap().take();
        let _ = expected_json.get_mut("allowed_values").unwrap().take();

        assert_eq!(expected_json, json);
    }

    #[test]
    fn test_service_line_setter() {
        let fbp = fbp_results();

        let service_line_setter = CustomPropertySetter::from_fbp_service_line(fbp.service_lines());
        let mut json = serde_json::to_value(&service_line_setter).unwrap();

        let mut expected_json = serde_json::to_value(serde_json::json!({
            "property_name": "service_line",
            "value_type": "single_select",
            "required": false,
            "default_value": null,
            "description": "The Service Line",
            "allowed_values": fbp.service_lines(),
            "values_editable_by": "org_and_repo_actors",
        }))
        .unwrap();

        // Check all allowed_values exist in the output object
        fbp.service_lines()
            .iter()
            .flat_map(serde_json::to_value)
            .for_each(|sl| {
                assert!(json
                    .get("allowed_values")
                    .expect("allowed_values should have items")
                    .as_array()
                    .expect("should be array")
                    .contains(&sl))
            });

        // Remove allowed values due to array ordering Eq
        let _ = json.get_mut("allowed_values").unwrap().take();
        let _ = expected_json.get_mut("allowed_values").unwrap().take();

        assert_eq!(expected_json, json);
    }

    #[test]
    fn test_product_setter() {
        let product_setter: CustomPropertySetter = CustomPropertySetter::from_fbp_product();

        let json = serde_json::to_value(&product_setter).unwrap();

        //let allowed_values: Vec<&str> = vec![];
        let expected_json = serde_json::to_value(serde_json::json!({
            "property_name": "product",
            "value_type": "string",
            "required": false,
            "default_value": null,
            "description": "Product",
            "allowed_values": null,
            "values_editable_by": "org_and_repo_actors",
        }))
        .unwrap();
        assert_eq!(json, expected_json);
    }
}
