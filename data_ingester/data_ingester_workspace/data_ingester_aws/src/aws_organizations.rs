use data_ingester_splunk::splunk::ToHecEvents;
use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Organization {
    /// <p>The unique identifier (ID) of an organization.</p>
    /// <p>The <a href="http://wikipedia.org/wiki/regex">regex pattern</a> for an organization ID string requires "o-" followed by from 10 to 32 lowercase letters or digits.</p>
    pub id: ::std::option::Option<::std::string::String>,
    /// <p>The Amazon Resource Name (ARN) of an organization.</p>
    /// <p>For more information about ARNs in Organizations, see <a href="https://docs.aws.amazon.com/service-authorization/latest/reference/list_awsorganizations.html#awsorganizations-resources-for-iam-policies">ARN Formats Supported by Organizations</a> in the <i>Amazon Web Services Service Authorization Reference</i>.</p>
    pub arn: ::std::option::Option<::std::string::String>,
    /// <p>Specifies the functionality that currently is available to the organization. If set to "ALL", then all features are enabled and policies can be applied to accounts in the organization. If set to "CONSOLIDATED_BILLING", then only consolidated billing functionality is available. For more information, see <a href="https://docs.aws.amazon.com/organizations/latest/userguide/orgs_manage_org_support-all-features.html">Enabling all features in your organization</a> in the <i>Organizations User Guide</i>.</p>
    pub feature_set: ::std::option::Option<String>,
    /// <p>The Amazon Resource Name (ARN) of the account that is designated as the management account for the organization.</p>
    /// <p>For more information about ARNs in Organizations, see <a href="https://docs.aws.amazon.com/service-authorization/latest/reference/list_awsorganizations.html#awsorganizations-resources-for-iam-policies">ARN Formats Supported by Organizations</a> in the <i>Amazon Web Services Service Authorization Reference</i>.</p>
    pub master_account_arn: ::std::option::Option<::std::string::String>,
    /// <p>The unique identifier (ID) of the management account of an organization.</p>
    /// <p>The <a href="http://wikipedia.org/wiki/regex">regex pattern</a> for an account ID string requires exactly 12 digits.</p>
    pub master_account_id: ::std::option::Option<::std::string::String>,
    /// <p>The email address that is associated with the Amazon Web Services account that is designated as the management account for the organization.</p>
    pub master_account_email: ::std::option::Option<::std::string::String>,
    /// <important>
    /// <p>Do not use. This field is deprecated and doesn't provide complete information about the policies in your organization.</p>
    /// </important>
    /// <p>To determine the policies that are enabled and available for use in your organization, use the <code>ListRoots</code> operation instead.</p>
    pub available_policy_types: ::std::option::Option<::std::vec::Vec<PolicyTypeSummary>>,
    pub in_organization: Option<bool>,
    pub account_id: Option<String>,
}

impl Default for Organization {
    fn default() -> Self {
        Self {
            id: None,
            arn: None,
            feature_set: None,
            master_account_arn: None,
            master_account_id: None,
            master_account_email: None,
            available_policy_types: None,
            in_organization: Some(false),
            account_id: None,
        }
    }
}

impl From<aws_sdk_organizations::types::Organization> for Organization {
    fn from(value: aws_sdk_organizations::types::Organization) -> Self {
        let in_organization = value.master_account_arn().map(|_arn| true);
        Self {
            id: value.id,
            arn: value.arn,
            feature_set: value.feature_set.map(|fs| fs.as_str().to_string()),
            master_account_arn: value.master_account_arn.clone(),
            master_account_id: value.master_account_id,
            master_account_email: value.master_account_email,
            available_policy_types: value
                .available_policy_types
                .map(|vec| vec.into_iter().map(|apt| apt.into()).collect()),
            in_organization,
            account_id: None,
        }
    }
}

impl ToHecEvents for &Organization {
    type Item = Self;

    fn source(&self) -> &str {
        "organizations_describeOrganization"
    }

    fn sourcetype(&self) -> &str {
        "ssphp:aws:json"
    }

    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        Box::new(std::iter::once(self))
    }
}

#[derive(Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PolicyTypeSummary {
    /// <p>The name of the policy type.</p>
    pub r#type: ::std::option::Option<String>,
    /// <p>The status of the policy type as it relates to the associated root. To attach a policy of the specified type to a root or to an OU or account in that root, it must be available in the organization and enabled for that root.</p>
    pub status: ::std::option::Option<String>,
}

impl From<aws_sdk_organizations::types::PolicyTypeSummary> for PolicyTypeSummary {
    fn from(value: aws_sdk_organizations::types::PolicyTypeSummary) -> Self {
        Self {
            r#type: value.r#type.map(|t| t.as_str().to_string()),
            status: value.status.map(|s| s.as_str().to_string()),
        }
    }
}
