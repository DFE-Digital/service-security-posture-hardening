use std::marker::PhantomData;

use serde::{Deserialize, Serialize};

/// https://docs.github.com/en/rest/orgs/custom-properties?apiVersion=2022-11-28#create-or-update-custom-properties-for-an-organization
#[derive(Debug, Deserialize, Serialize)]
pub struct CustomProperterySetter<V, S>
where
    V: AsRef<[S]>,
    S: AsRef<str>,
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
    _phantom_data: PhantomData<S>,
}

impl<V: AsRef<[S]>, S: AsRef<str>> CustomProperterySetter<V, S> {
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
        CustomProperterySetter::new("product", Some("Product"), false, ValueType::String)
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

#[cfg(feature = "live_tests")]
#[cfg(test)]
mod live_tests {
    // use super::*;
    // use anyhow::Context;
    // use anyhow::Result;
    // use octocrab::Octocrab;

    // fn octocrab_client() -> Result<Octocrab> {
    //     let token = std::env::var("GITHUB_TOKEN").expect("GITHUB_TOKEN env variable is required");

    //     let octocrab = Octocrab::builder().personal_token(token).build()?;
    //     Ok(octocrab)
    // }

    // #[tokio::test]
    // async fn test_custom_property_setter_output() -> Result<()> {
    //     let cps1 = {
    //         let mut cps = CustomProperterySetter::<Vec<_>, &str>::new(
    //             "1_portfolio".to_string(),
    //             "description".into(),
    //             false,
    //             ValueType::SingleSelect,
    //         );
    //         //cps.default_value = Some(DefaultValue::String("FOO".into()));
    //         cps.values_editable_by = Some(ValuesEditableBy::OrgAndRepoActors);
    //         cps.allowed_values = Some(vec![
    //             "Afghanistan",
    //             "Albania",
    //             "Algeria",
    //             "Andorra",
    //             "Angola",
    //             "Antigua and Barbuda",
    //             "Argentina",
    //             "Armenia",
    //             "Australia",
    //             "Austria",
    //             "Azerbaijan",
    //             "B",
    //             "The Bahamas",
    //             "Bahrain",
    //             "Bangladesh",
    //             "Barbados",
    //             "Belarus",
    //             "Belgium",
    //             "Belize",
    //             "Benin",
    //             "Bhutan",
    //             "Bolivia",
    //             "Bosnia and Herzegovina",
    //             "Botswana",
    //             "Brazil",
    //             "Brunei",
    //             "Bulgaria",
    //             "Burkina Faso",
    //             "Burundi",
    //             "C",
    //             "Cabo Verde",
    //             "Cambodia",
    //             "Cameroon",
    //             "Canada",
    //             "Central African Republic",
    //             "Chad",
    //             "Chile",
    //             "China",
    //             "Colombia",
    //             "Comoros",
    //             "Congo, Democratic Republic of the",
    //             "Congo, Republic of the",
    //             "Costa Rica",
    //             "Croatia",
    //             "Cuba",
    //             "Cyprus",
    //             "Czech Republic",
    //             "D",
    //             "Denmark",
    //             "Djibouti",
    //             "Dominica",
    //             "Dominican Republic",
    //             "E",
    //             "East Timor (Timor-Leste)",
    //             "Ecuador",
    //             "Egypt",
    //             "El Salvador",
    //             "Equatorial Guinea",
    //             "Eritrea",
    //             "Estonia",
    //             "Eswatini",
    //             "Ethiopia",
    //         ]);
    //         cps
    //     };
    //     let cps2 = {
    //         let mut cps = CustomProperterySetter::new(
    //             "2_service_line".to_string(),
    //             Some("description"),
    //             false,
    //             ValueType::SingleSelect,
    //         );
    //         //cps.default_value = Some(DefaultValue::String("FOO"));
    //         cps.values_editable_by = Some(ValuesEditableBy::OrgAndRepoActors);
    //         cps.allowed_values = Some(vec![
    //             "Mastador",
    //             "Mastiff",
    //             "Meagle",
    //             "Meerkat",
    //             "Mexican Free-Tailed Bat",
    //             "Miki",
    //             "Mini Labradoodle",
    //             "Miniature Bull Terrier",
    //             "Miniature Pinscher",
    //             "Mink",
    //             "Minke Whale",
    //             "Mole",
    //             "Mongoose",
    //             "Mongrel",
    //             "Monkey",
    //             "Moorhen",
    //             "Moose",
    //             "Morkie",
    //             "Moscow Watchdog",
    //             "Mountain Cur",
    //             "Mountain Feist",
    //             "Mountain Gorilla",
    //             "Mountain Lion",
    //             "Mouse",
    //             "Mudi",
    //             "Mule",
    //             "Mule Deer",
    //             "Muntjac",
    //             "Musk Deer",
    //             "Muskox",
    //             "Muskrat",
    //             "Nabarlek",
    //             "Naked Mole Rat",
    //             "Narwhal",
    //             "Neanderthal",
    //             "Neapolitan Mastiff",
    //             "Nebelung",
    //             "Netherland Dwarf Rabbit",
    //             "Newfoundland",
    //             "Newfypoo",
    //             "Nigerian Goat",
    //             "Nilgai",
    //             "Norfolk Terrier",
    //             "North American Black Bear",
    //             "Northern Fur Seal",
    //             "Northern Inuit Dog",
    //             "Norwegian Buhund",
    //             "Norwegian Elkhound",
    //             "Norwegian Forest",
    //             "Norwegian Lundehund",
    //             "Norwich Terrier",
    //             "Nova Scotia Duck Tolling Retriever",
    //             "Nubian Goat",
    //             "Numbat",
    //             "Nutria",
    //             "Nyala",
    //             "Ocelot",
    //             "Okapi",
    //             "Old English Sheepdog",
    //             "Olingo",
    //             "Olive Baboon",
    //             "Onager",
    //             "Opossum",
    //             "Orangutan",
    //             "Oribi",
    //             "Otter",
    //             "Otterhound",
    //         ]);
    //         cps
    //     };

    //     let cps3 = {
    //         let mut cps = CustomProperterySetter::new(
    //             "3_product".to_string(),
    //             Some("description"),
    //             false,
    //             ValueType::String,
    //         );
    //         //cps.default_value = Some(DefaultValue::String("FOO"));
    //         cps.values_editable_by = Some(ValuesEditableBy::OrgAndRepoActors);
    //         cps
    //     };

    //     let octocrab = octocrab_client()?;
    //     for cps in [cps1, cps2, cps3] {
    //         let response = octocrab
    //             ._put(
    //                 format!(
    //                     "https://api.github.com/orgs/403ind/properties/schema/{}",
    //                     cps.property_name
    //                 ),
    //                 Some(&cps),
    //             )
    //             .await?;
    //         dbg!(&response);
    //         dbg!(&response.status());
    //         let body = response.collect().await?.to_bytes().to_vec();
    //         dbg!(String::from_utf8(body).unwrap());
    //     }

    //     let cps4 = {
    //         let mut cps: CustomProperterySetter::<Vec<_>, &str> = CustomProperterySetter::new(
    //             "product",
    //             Some("description"),
    //             false,
    //             ValueType::String,
    //         );
    //         //cps.default_value = Some(DefaultValue::String("FOO"));
    //         cps.values_editable_by = Some(ValuesEditableBy::OrgAndRepoActors);
    //         cps
    //     };
    //     let response = octocrab
    //         ._put(
    //             format!("https://api.github.com/orgs/403ind/properties/schema/7%20product"),
    //             Some(&cps4),
    //         )
    //         .await?;
    //     dbg!(&response);
    //     dbg!(&response.status());
    //     let body = response.collect().await?.to_bytes().to_vec();
    //     dbg!(String::from_utf8(body).unwrap());

    //     assert!(false);
    //     Ok(())
    // }
}

#[cfg(test)]
mod test {
    use data_ingester_financial_business_partners::fbp_results::{FbpResult, FbpResults};

    use crate::custom_properties::{
        CustomProperterySetter, DefaultValue, ValueType, ValuesEditableBy,
    };

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
    fn test_portfolio_setter() {
        let fbp = fbp_results();

        let portfolio_setter = CustomProperterySetter::from_fbp_portfolio(fbp.portfolios());
        let json = serde_json::to_value(&portfolio_setter).unwrap();

        let expected_json = serde_json::to_value(serde_json::json!({
            "property_name": "portfolio",
            "value_type": "multi_select",
            "required": false,
            "default_value": null,
            "description": "The portfolio",
            "allowed_values": fbp.portfolios(),
            "values_editable_by": "org_and_repo_actors",
        }))
        .unwrap();
        assert_eq!(expected_json, json);
    }

    #[test]
    fn test_service_line_setter() {
        let fbp = fbp_results();

        let service_line_setter =
            CustomProperterySetter::from_fbp_service_line(fbp.service_lines());
        let json = serde_json::to_value(&service_line_setter).unwrap();

        let expected_json = serde_json::to_value(serde_json::json!({
            "property_name": "service_line",
            "value_type": "multi_select",
            "required": false,
            "default_value": null,
            "description": "The service line",
            "allowed_values": fbp.service_lines(),
            "values_editable_by": "org_and_repo_actors",
        }))
        .unwrap();
        assert_eq!(expected_json, json);
    }

    #[tokio::test]
    async fn test_custom_property_setter_output() {
        let cps = {
            let mut cps = CustomProperterySetter::<Vec<_>, &str>::new(
                "foo",
                Some("description"),
                false,
                ValueType::SingleSelect,
            );
            cps.default_value = Some(DefaultValue::String("FOO".into()));
            cps.values_editable_by = Some(ValuesEditableBy::OrgActors);
            cps.allowed_values = Some(vec!["foo", "bar"]);
            cps
        };
        let json = serde_json::to_string_pretty(&cps).unwrap();
        let expected_json = serde_json::to_value(serde_json::json!({
            "property_name": "service_line",
            "value_type": "multi_select",
            "required": false,
            "default_value": null,
            "description": "The service line",
            "allowed_values": cps.allowed_values,
            "values_editable_by": "org_and_repo_actors",
        }))
        .unwrap();

        println!("{}", json);
        assert_eq!(json, expected_json);
    }
}
