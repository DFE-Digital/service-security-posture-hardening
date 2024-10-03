use anyhow::Context;

use octocrab::models::teams::RequestedReviewers;
use serde::{Deserialize, Serialize};

/// https://docs.github.com/en/rest/orgs/custom-properties?apiVersion=2022-11-28#create-or-update-custom-properties-for-an-organization
#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct CustomProperterySetter {
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
    allowed_values: Option<Vec<String>>,

    // Who can edit the values of the property
    values_editable_by: Option<ValuesEditableBy>,
}

impl CustomProperterySetter {
    fn new<S: Into<String>>(property_name: S, value_type: ValueType, required: bool) -> Self {
        Self {
            property_name: property_name.into(),
            //url: None,
            value_type,
            required,
            default_value: None,
            description: None,
            allowed_values: None,
            values_editable_by: None,
        }
    }

    pub(crate) fn property_name(&self) -> &str {
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

mod live_tests {
    use super::*;
    use anyhow::Result;
    use http_body_util::BodyExt;
    use octocrab::Octocrab;

    fn octocrab_client() -> Result<Octocrab> {
        let token = std::env::var("GITHUB_TOKEN").expect("GITHUB_TOKEN env variable is required");

        let octocrab = Octocrab::builder().personal_token(token).build()?;
        Ok(octocrab)
    }

    #[tokio::test]
    async fn test_custom_property_setter_output() -> Result<()> {
        let cps1 = {
            let mut cps = CustomProperterySetter::new(
                "1_portfolio".to_string(),
                ValueType::SingleSelect,
                false,
            );
            //cps.default_value = Some(DefaultValue::String("FOO".into()));
            cps.values_editable_by = Some(ValuesEditableBy::OrgAndRepoActors);
            cps.allowed_values = Some(vec![
                "Afghanistan".into(),
                "Albania".into(),
                "Algeria".into(),
                "Andorra".into(),
                "Angola".into(),
                "Antigua and Barbuda".into(),
                "Argentina".into(),
                "Armenia".into(),
                "Australia".into(),
                "Austria".into(),
                "Azerbaijan".into(),
                "B".into(),
                "The Bahamas".into(),
                "Bahrain".into(),
                "Bangladesh".into(),
                "Barbados".into(),
                "Belarus".into(),
                "Belgium".into(),
                "Belize".into(),
                "Benin".into(),
                "Bhutan".into(),
                "Bolivia".into(),
                "Bosnia and Herzegovina".into(),
                "Botswana".into(),
                "Brazil".into(),
                "Brunei".into(),
                "Bulgaria".into(),
                "Burkina Faso".into(),
                "Burundi".into(),
                "C".into(),
                "Cabo Verde".into(),
                "Cambodia".into(),
                "Cameroon".into(),
                "Canada".into(),
                "Central African Republic".into(),
                "Chad".into(),
                "Chile".into(),
                "China".into(),
                "Colombia".into(),
                "Comoros".into(),
                "Congo, Democratic Republic of the".into(),
                "Congo, Republic of the".into(),
                "Costa Rica".into(),
                "Croatia".into(),
                "Cuba".into(),
                "Cyprus".into(),
                "Czech Republic".into(),
                "D".into(),
                "Denmark".into(),
                "Djibouti".into(),
                "Dominica".into(),
                "Dominican Republic".into(),
                "E".into(),
                "East Timor (Timor-Leste)".into(),
                "Ecuador".into(),
                "Egypt".into(),
                "El Salvador".into(),
                "Equatorial Guinea".into(),
                "Eritrea".into(),
                "Estonia".into(),
                "Eswatini".into(),
                "Ethiopia".into(),
            ]);
            cps
        };
        let cps2 = {
            let mut cps = CustomProperterySetter::new(
                "2_service_line".to_string(),
                ValueType::SingleSelect,
                false,
            );
            //cps.default_value = Some(DefaultValue::String("FOO".into()));
            cps.values_editable_by = Some(ValuesEditableBy::OrgAndRepoActors);
            cps.allowed_values = Some(vec![
                "Mastador".into(),
                "Mastiff".into(),
                "Meagle".into(),
                "Meerkat".into(),
                "Mexican Free-Tailed Bat".into(),
                "Miki".into(),
                "Mini Labradoodle".into(),
                "Miniature Bull Terrier".into(),
                "Miniature Pinscher".into(),
                "Mink".into(),
                "Minke Whale".into(),
                "Mole".into(),
                "Mongoose".into(),
                "Mongrel".into(),
                "Monkey".into(),
                "Moorhen".into(),
                "Moose".into(),
                "Morkie".into(),
                "Moscow Watchdog".into(),
                "Mountain Cur".into(),
                "Mountain Feist".into(),
                "Mountain Gorilla".into(),
                "Mountain Lion".into(),
                "Mouse".into(),
                "Mudi".into(),
                "Mule".into(),
                "Mule Deer".into(),
                "Muntjac".into(),
                "Musk Deer".into(),
                "Muskox".into(),
                "Muskrat".into(),
                "Nabarlek".into(),
                "Naked Mole Rat".into(),
                "Narwhal".into(),
                "Neanderthal".into(),
                "Neapolitan Mastiff".into(),
                "Nebelung".into(),
                "Netherland Dwarf Rabbit".into(),
                "Newfoundland".into(),
                "Newfypoo".into(),
                "Nigerian Goat".into(),
                "Nilgai".into(),
                "Norfolk Terrier".into(),
                "North American Black Bear".into(),
                "Northern Fur Seal".into(),
                "Northern Inuit Dog".into(),
                "Norwegian Buhund".into(),
                "Norwegian Elkhound".into(),
                "Norwegian Forest".into(),
                "Norwegian Lundehund".into(),
                "Norwich Terrier".into(),
                "Nova Scotia Duck Tolling Retriever".into(),
                "Nubian Goat".into(),
                "Numbat".into(),
                "Nutria".into(),
                "Nyala".into(),
                "Ocelot".into(),
                "Okapi".into(),
                "Old English Sheepdog".into(),
                "Olingo".into(),
                "Olive Baboon".into(),
                "Onager".into(),
                "Opossum".into(),
                "Orangutan".into(),
                "Oribi".into(),
                "Otter".into(),
                "Otterhound".into(),
            ]);
            cps
        };

        let cps3 = {
            let mut cps =
                CustomProperterySetter::new("3_product".to_string(), ValueType::String, false);
            //cps.default_value = Some(DefaultValue::String("FOO".into()));
            cps.values_editable_by = Some(ValuesEditableBy::OrgAndRepoActors);
            cps
        };

        let octocrab = octocrab_client()?;
        for cps in [cps1, cps2, cps3] {
            let response = octocrab
                ._put(
                    format!(
                        "https://api.github.com/orgs/403ind/properties/schema/{}",
                        cps.property_name
                    ),
                    Some(&cps),
                )
                .await?;
            dbg!(&response);
            dbg!(&response.status());
            let body = response.collect().await?.to_bytes().to_vec();
            dbg!(String::from_utf8(body));
        }

        let cps4 = {
            let mut cps =
                CustomProperterySetter::new("product".to_string(), ValueType::String, false);
            //cps.default_value = Some(DefaultValue::String("FOO".into()));
            cps.values_editable_by = Some(ValuesEditableBy::OrgAndRepoActors);
            cps
        };
        let response = octocrab
            ._put(
                format!("https://api.github.com/orgs/403ind/properties/schema/7%20product"),
                Some(&cps4),
            )
            .await?;
        dbg!(&response);
        dbg!(&response.status());
        let body = response.collect().await?.to_bytes().to_vec();
        dbg!(String::from_utf8(body));

        assert!(false);
        Ok(())
    }
}
#[tokio::test]
async fn test_custom_property_setter_output() {
    let cps = {
        let mut cps =
            CustomProperterySetter::new("foo".to_string(), ValueType::SingleSelect, false);
        cps.default_value = Some(DefaultValue::String("FOO".into()));
        cps.values_editable_by = Some(ValuesEditableBy::OrgActors);
        cps.allowed_values = Some(vec!["foo".into(), "bar".into()]);
        cps
    };
    let json = serde_json::to_string_pretty(&cps).unwrap();
    println!("{}", json);
    // assert!(false);
}
