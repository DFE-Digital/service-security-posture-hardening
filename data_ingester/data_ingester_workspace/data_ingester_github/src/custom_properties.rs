use std::marker::PhantomData;

use serde::{Deserialize, Serialize};

/// https://docs.github.com/en/rest/orgs/custom-properties?apiVersion=2022-11-28#create-or-update-custom-properties-for-an-organization
#[derive(Debug, Deserialize, Serialize)]
pub struct CustomProperterySetter<V, S>
where
    V: AsRef<[S]>,
    S: AsRef<str> + std::fmt::Debug,
{
    // The name of the property
    property_name: String,

    //The URL that can be used to fetch, update, or delete info about this property via the API.
    //url: Option<String>,

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
    allowed_values: Option<V>,

    // Who can edit the values of the property
    values_editable_by: Option<ValuesEditableBy>,
    #[serde(skip)]
    _phantom_data: PhantomData<S>,
}

impl<V: AsRef<[S]>, S: AsRef<str> + std::fmt::Debug> CustomProperterySetter<V, S> {
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
            _phantom_data: PhantomData,
        }
    }

    pub fn new_single_select<N: Into<String>, D: Into<String>>(
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
            allowed_values: Some(allowed_values),
            values_editable_by: Some(ValuesEditableBy::OrgAndRepoActors),
            _phantom_data: PhantomData,
        }
    }

    pub fn from_fbp_portfolio(allowed_values: V) -> Self {
        CustomProperterySetter::new_single_select(
            "portfolio",
            Some("The portfolio"),
            false,
            allowed_values,
        )
    }

    pub fn from_fbp_service_line(allowed_values: V) -> Self {
        CustomProperterySetter::new_single_select(
            "service_line",
            Some("The service line"),
            false,
            allowed_values,
        )
    }

    pub fn from_fbp_product() -> Self {
        let mut product_setter =
            CustomProperterySetter::new("product", Some("Product"), false, ValueType::String);
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

#[cfg(test)]
mod test {
    use data_ingester_financial_business_partners::fbp_results::FbpResult;

    use crate::custom_properties::CustomProperterySetter;

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

        let portfolio_setter = CustomProperterySetter::from_fbp_portfolio(fbp.portfolios());
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
            .flat_map(|po| serde_json::to_value(po))
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

        let service_line_setter =
            CustomProperterySetter::from_fbp_service_line(fbp.service_lines());
        let mut json = serde_json::to_value(&service_line_setter).unwrap();

        let mut expected_json = serde_json::to_value(serde_json::json!({
            "property_name": "service_line",
            "value_type": "single_select",
            "required": false,
            "default_value": null,
            "description": "The service line",
            "allowed_values": fbp.service_lines(),
            "values_editable_by": "org_and_repo_actors",
        }))
        .unwrap();

        // Check all allowed_values exist in the output object
        fbp.service_lines()
            .iter()
            .flat_map(|sl| serde_json::to_value(sl))
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
        let product_setter: CustomProperterySetter<Vec<_>, &str> =
            CustomProperterySetter::from_fbp_product();

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
