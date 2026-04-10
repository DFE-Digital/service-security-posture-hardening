use std::sync::Arc;

use crate::github_response::{GithubResponse, GithubResponses};
use data_ingester_financial_business_partners::validator::{ValidationResult, Validator};
use data_ingester_splunk::splunk::ToHecEvents;
use itertools::{Either, Itertools};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::{error, info};

/// https://docs.github.com/en/rest/orgs/custom-properties?apiVersion=2022-11-28#create-or-update-custom-properties-for-an-organization
#[derive(Debug, Deserialize, Serialize)]
pub struct CustomPropertySetter {
    // An ordered list of the allowed values of the property. The property can have up to 200 allowed values.
    allowed_values: Option<Vec<String>>,

    // Default value of the property
    default_value: Option<DefaultValue>,

    // Short description of the property
    description: Option<String>,

    // The name of the property
    property_name: String,

    // Regex to validate
    #[serde(skip_serializing_if = "Option::is_none")]
    regex: Option<String>,

    // ??
    #[serde(skip_serializing_if = "Option::is_none")]
    require_explicit_values: Option<bool>,

    // Whether the property is required.
    required: bool,

    // The type of the value for the property
    // Can be one of: string, single_select, multi_select, true_false
    value_type: ValueType,

    // Who can edit the values of the property
    values_editable_by: Option<ValuesEditableBy>,
}

static SERVICE_LINE_CLEANER_DATA: [(&str, &str); 3] = [
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
    (
        "H102 Hub Management - Business Services Log Analytics",
        // NBSP             ^
        "H102 Hub Management - Business Services Log Analytics",
    ),
];

#[derive(Default)]
pub struct ServiceLineCleaner();

impl ServiceLineCleaner {
    pub fn allowed_values_cleaner_to_github<'value, S: AsRef<str>>(
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

    pub fn allowed_values_cleaner_from_github<'value, S: AsRef<str>>(
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
            regex: None,
            require_explicit_values: None,
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
            regex: None,
            require_explicit_values: None,
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

    // Transform and values that contain multibyte UTF8 chars into an ASCII equivelent.
    pub fn service_line_cleaner<V: AsRef<[S]>, S: AsRef<str> + std::fmt::Debug>(
        allowed_values: V,
    ) -> Vec<String> {
        let service_line_cleaner = ServiceLineCleaner::default();
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
        allowed_value_strings
    }

    pub fn from_fbp_service_line<V: AsRef<[S]>, S: AsRef<str> + std::fmt::Debug>(
        allowed_values: V,
    ) -> Self {
        let allowed_value_strings = Self::service_line_cleaner(allowed_values);

        CustomPropertySetter::new_single_select(
            "service_line",
            Some("The Service Line"),
            false,
            allowed_value_strings,
        )
    }

    pub fn from_fbp_product<V: AsRef<[S]>, S: AsRef<str> + std::fmt::Debug>(
        allowed_values: V,
    ) -> Self {
        let mut product_setter =
            CustomPropertySetter::new("product", Some("Product"), false, ValueType::String);
        product_setter.values_editable_by = Some(ValuesEditableBy::OrgAndRepoActors);

        let regex = {
            let allowed_value_strings = Self::service_line_cleaner(allowed_values);
            let regex = Self::product_names_regex(&allowed_value_strings);
            match Self::validate_product_regex(&allowed_value_strings, &regex) {
                Ok(true) => {
                    info!(regex, "Setting GitHub custom properties Product regex");
                    Some(regex)
                }
                Ok(false) => {
                    error!(
                        regex,
                        "Regex validation failed, not setting validation regex for FBP Products in GitHub custom properties"
                    );
                    None
                }
                Err(err) => {
                    let err = format!("{:#?}", err);
                    error!(
                        regex,
                        err,
                        "Unable to compile regex for FBP validation. Not setting validation regex"
                    );
                    None
                }
            }
        };
        product_setter.regex = regex;

        product_setter
    }

    pub fn property_name(&self) -> &str {
        &self.property_name
    }

    /// Build a regex that matches all the given values. Try to escape any chars that might be used as regex control characters.
    fn product_names_regex<V: AsRef<[S]>, S: AsRef<str> + std::fmt::Debug>(
        allowed_values: V,
    ) -> String {
        let mut names_iter = allowed_values
            .as_ref()
            .iter()
            .map(|name| name.as_ref().replace(r#"\"#, r#"\\"#))
            .map(|name| name.replace("(", r"\("))
            .map(|name| name.replace(")", r"\)"))
            .map(|name| name.replace("[", r"\["))
            .map(|name| name.replace("]", r"\]"))
            .map(|name| name.replace(".", r"\."))
            .map(|name| name.replace("/", r"\/"))
            .map(|name| name.replace(r#"-"#, r#"\-"#))
            .map(|name| name.replace(r#" "#, r#"\ "#))
            .map(|name| name.replace(r#"&"#, r#"\&"#))
            .map(|name| name.replace(r#"|"#, r#"\|"#))
            .map(|name| name.replace(r#"$"#, r#"\$"#))
            .map(|name| name.replace(r#"^"#, r#"\^"#));
        format!("^({})$", &names_iter.join("|"))
    }

    // Check all values can be matched by the regex. Used before setting the regex.
    fn validate_product_regex<V: AsRef<[S]>, S: AsRef<str> + std::fmt::Debug>(
        allowed_values: V,
        regex: &str,
    ) -> Result<bool, regex::Error> {
        let regex = regex::Regex::new(regex)?;
        let result = allowed_values
            .as_ref()
            .iter()
            .all(|value| regex.is_match(value.as_ref()));
        Ok(result)
    }

    /// Extract values that can't be removed from a CustomProprety
    /// because they are in use and add them back to the setter.
    ///
    ///
    /// Given an error message in the form of:
    ///
    /// {
    ///     "message": "Unable to save 'service_line' because you can't delete options that are in use: 'CSC Social Worker Training, Development and Leadership', 'Infrastructure and Platforms', 'Markets, Strategy & Workforce', 'Teacher Services' referenced by 15 Repositories."
    ///     "status": 422
    /// }
    ///
    pub fn update_allowed_values_from_422(&mut self, error_value: Value) -> anyhow::Result<()> {
        let error: ErrorMessage = serde_json::from_value(error_value)?;
        if error.status != "422" {
            anyhow::bail!("Not a 422 status code");
        }
        // Split the error message on the first ":" to skip the name/key of the CustomProperty
        let hay = error.message.split(":").nth(1).unwrap_or_default();

        // Match anything inside single quotes
        let rex = Regex::new(r#"'([^']+?)'"#)?;

        if let Some(allowed_values) = self.allowed_values.as_mut() {
            rex.captures_iter(hay)
                .filter_map(|capture| capture.get(1).map(|s| s.as_str().to_string()))
                .for_each(|value| allowed_values.push(value));
        }
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
struct ErrorMessage {
    message: String,
    status: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ValueType {
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
pub struct CustomProperties {
    pub custom_properties: Vec<CustomProperty>,
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
pub struct CustomProperty {
    repository_id: u64,
    pub repository_name: String,
    repository_full_name: String,
    properties: Vec<Property>,
    #[serde(skip_deserializing)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub validation_errors: Option<ValidationResult>,
}

impl CustomProperty {
    fn clean(&mut self) {
        let service_line_cleaner = ServiceLineCleaner::default();
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
pub struct Property {
    pub property_name: String,
    pub value: Option<VecOrString>,
}

impl Property {
    pub fn new_single_value<S1: Into<String>, S2: Into<String>>(
        property_name: S1,
        value: S2,
    ) -> Self {
        Self {
            property_name: property_name.into(),
            value: Some(VecOrString::String(value.into())),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum VecOrString {
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

#[derive(Debug, Deserialize, Serialize)]
pub struct SetOrgRepoCustomProperties {
    pub repository_names: Vec<String>,
    pub properties: Vec<Property>,
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
    use serde_json::json;

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
        let products = ["foo", "bar", "baz"];
        let product_setter: CustomPropertySetter = CustomPropertySetter::from_fbp_product(products);

        let json = serde_json::to_value(&product_setter).unwrap();

        let expected_json = serde_json::to_value(serde_json::json!({
            "property_name": "product",
            "value_type": "string",
            "required": false,
            "default_value": null,
            "description": "Product",
            "allowed_values": null,
            "values_editable_by": "org_and_repo_actors",
            "regex": "^(foo|bar|baz)$",
        }))
        .unwrap();
        assert_eq!(json, expected_json);
    }

    #[test]
    fn product_names_regex_test() {
        let names = [
            "foo",
            "bar",
            "baz",
            r"rparen(",
            r"lparen)",
            r"rsquare[",
            r"lsquare]",
            r"dot.",
            r"fslash/",
            r#"bslash\"#,
            r"hypen-",
            r"space ",
            r"amber&",
            r"pipe|",
            r"hat^",
            r"dollar$",
        ];
        let template = CustomPropertySetter::product_names_regex(names);
        let regex = regex::Regex::new(&template).unwrap();
        for name in names {
            assert!(regex.is_match(name));
        }
    }

    #[test]
    fn test_custom_properties_update_from_422() {
        let error_message = json!({
            "status": "422",
            "message": "Unable to save 'service_line' because you can't delete options that are in use: 'CSC Social Worker Training, Development and Leadership', 'Infrastructure and Platforms', 'Markets, Strategy & Workforce', 'Teacher Services' referenced by 15 Repositories."
        });

        let fbp = fbp_results();
        let mut service_line_setter =
            CustomPropertySetter::from_fbp_service_line(fbp.service_lines());

        service_line_setter
            .update_allowed_values_from_422(error_message)
            .unwrap();

        let allowed_values = service_line_setter.allowed_values.as_ref().unwrap();

        assert!(allowed_values
            .contains(&"CSC Social Worker Training, Development and Leadership".to_string()));
        assert!(allowed_values.contains(&"Infrastructure and Platforms".to_string()));
        assert!(allowed_values.contains(&"Markets, Strategy & Workforce".to_string()));
        assert!(allowed_values.contains(&"Teacher Services".to_string()));
        assert_eq!(allowed_values.len(), 7);
    }
}
