use serde::Serialize;

use data_ingester_splunk::splunk::ToHecEvents;
#[derive(
    ::std::clone::Clone,
    ::std::cmp::PartialEq,
    ::std::default::Default,
    ::std::fmt::Debug,
    Serialize,
)]
pub struct DescribeHubOutput {
    pub(crate) hub_arn: ::std::option::Option<::std::string::String>,
    pub(crate) subscribed_at: ::std::option::Option<::std::string::String>,
    pub(crate) auto_enable_controls: ::std::option::Option<bool>,
    pub(crate) control_finding_generator: ::std::option::Option<String>,
}

impl ToHecEvents for &DescribeHubOutput {
    type Item = Self;

    fn source(&self) -> &str {
        "securityhub_DescribeHub"
    }

    fn sourcetype(&self) -> &str {
        "ssphp:aws:json"
    }

    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        Box::new(std::iter::once(self))
    }
}

impl From<aws_sdk_securityhub::operation::describe_hub::DescribeHubOutput> for DescribeHubOutput {
    fn from(value: aws_sdk_securityhub::operation::describe_hub::DescribeHubOutput) -> Self {
        Self {
            hub_arn: value.hub_arn,
            subscribed_at: value.subscribed_at,
            auto_enable_controls: value.auto_enable_controls,
            control_finding_generator: value
                .control_finding_generator
                .map(|cfg| cfg.as_str().to_owned()),
        }
    }
}
