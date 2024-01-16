use serde::Serialize;

use crate::splunk::ToHecEvents;

#[derive(::std::clone::Clone, ::std::cmp::PartialEq, Serialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct AlternateContact {
    /// <p>The name associated with this alternate contact.</p>
    pub name: ::std::option::Option<::std::string::String>,
    /// <p>The title associated with this alternate contact.</p>
    pub title: ::std::option::Option<::std::string::String>,
    /// <p>The email address associated with this alternate contact.</p>
    pub email_address: ::std::option::Option<::std::string::String>,
    /// <p>The phone number associated with this alternate contact.</p>
    pub phone_number: ::std::option::Option<::std::string::String>,
    /// <p>The type of alternate contact.</p>
    pub alternate_contact_type: ::std::option::Option<String>,
}

impl From<aws_sdk_account::types::AlternateContact> for AlternateContact {
    fn from(value: aws_sdk_account::types::AlternateContact) -> Self {
        Self {
            name: value.name,
            title: value.title,
            email_address: value.email_address,
            phone_number: value.phone_number,
            alternate_contact_type: value
                .alternate_contact_type
                .map(|act| act.as_str().to_owned()),
        }
    }
}

impl ToHecEvents for &AlternateContact {
    type Item = Self;

    fn source(&self) -> &str {
        "accounts_GetAlternateContact"
    }

    fn sourcetype(&self) -> &str {
        "ssphp:aws:json"
    }

    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        Box::new(std::iter::once(self))
    }
}
