use serde::{Deserialize, Serialize};

use data_ingester_splunk::splunk::ToHecEvents;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AwsPolicy {
    #[serde(rename = "Version")]
    pub version: String,
    #[serde(rename = "Statement")]
    pub statement: Statement,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(untagged)]
pub enum Statement {
    StatementVec(Vec<StatementElement>),
    StatementElement(StatementElement),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatementElement {
    #[serde(rename = "Effect")]
    pub effect: String,
    #[serde(rename = "Action")]
    pub action: Action,
    #[serde(rename = "Resource")]
    pub resource: Resource,
    #[serde(rename = "Sid")]
    pub sid: Option<String>,
    // #[serde(rename = "Condition")]
    // pub condition: Option<Condition>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(untagged)]
pub enum Action {
    ActionVec(Vec<String>),
    ActionSingle(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(untagged)]
pub enum Resource {
    ResourceVec(Vec<String>),
    ResourceSingle(String),
}

impl StatementElement {
    pub fn contains_full_permissions(&self) -> bool {
        if self.effect != "Allow" {
            return false;
        }
        // This needs to understand ARNs better
        let all_resources = match &self.resource {
            Resource::ResourceVec(vec) => vec.iter().any(|r| r == "*"),
            Resource::ResourceSingle(resource) => resource == "*",
        };

        let all_actions = match &self.action {
            Action::ActionVec(vec) => vec.iter().any(|action| action == "*" || action == "*:*"),
            Action::ActionSingle(action) => action == "*" || action == "*:*",
        };

        matches!((all_actions, all_resources), (true, true))
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct Policies {
    pub inner: Vec<Policy>,
}

impl From<Vec<aws_sdk_iam::types::Policy>> for Policies {
    fn from(value: Vec<aws_sdk_iam::types::Policy>) -> Self {
        Self {
            inner: value.into_iter().map(|u| u.into()).collect(),
        }
    }
}

impl ToHecEvents for &Policies {
    type Item = Policy;

    fn source(&self) -> &str {
        "iam_ListPolicies"
    }

    fn sourcetype(&self) -> &str {
        "ssphp:aws:json"
    }

    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        Box::new(self.inner.iter())
    }
    fn ssphp_run_key(&self) -> &str {
        "aws"
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct Policy {
    /// <p>The friendly name (not ARN) identifying the policy.</p>
    pub policy_name: ::std::option::Option<::std::string::String>,
    /// <p>The stable and unique string identifying the policy.</p>
    /// <p>For more information about IDs, see <a href="https://docs.aws.amazon.com/IAM/latest/UserGuide/Using_Identifiers.html">IAM identifiers</a> in the <i>IAM User Guide</i>.</p>
    pub policy_id: ::std::option::Option<::std::string::String>,
    /// <p>The Amazon Resource Name (ARN). ARNs are unique identifiers for Amazon Web Services resources.</p>
    /// <p>For more information about ARNs, go to <a href="https://docs.aws.amazon.com/general/latest/gr/aws-arns-and-namespaces.html">Amazon Resource Names (ARNs)</a> in the <i>Amazon Web Services General Reference</i>.</p>
    pub arn: ::std::option::Option<::std::string::String>,
    /// <p>The path to the policy.</p>
    /// <p>For more information about paths, see <a href="https://docs.aws.amazon.com/IAM/latest/UserGuide/Using_Identifiers.html">IAM identifiers</a> in the <i>IAM User Guide</i>.</p>
    pub path: ::std::option::Option<::std::string::String>,
    /// <p>The identifier for the version of the policy that is set as the default version.</p>
    pub default_version_id: ::std::option::Option<::std::string::String>,
    /// <p>The number of entities (users, groups, and roles) that the policy is attached to.</p>
    pub attachment_count: ::std::option::Option<i32>,
    /// <p>The number of entities (users and roles) for which the policy is used to set the permissions boundary.</p>
    /// <p>For more information about permissions boundaries, see <a href="https://docs.aws.amazon.com/IAM/latest/UserGuide/access_policies_boundaries.html">Permissions boundaries for IAM identities </a> in the <i>IAM User Guide</i>.</p>
    pub permissions_boundary_usage_count: ::std::option::Option<i32>,
    /// <p>Specifies whether the policy can be attached to an IAM user, group, or role.</p>
    pub is_attachable: bool,
    /// <p>A friendly description of the policy.</p>
    /// <p>This element is included in the response to the <code>GetPolicy</code> operation. It is not included in the response to the <code>ListPolicies</code> operation.</p>
    pub description: ::std::option::Option<::std::string::String>,
    /// <p>The date and time, in <a href="http://www.iso.org/iso/iso8601">ISO 8601 date-time format</a>, when the policy was created.</p>
    pub create_date: ::std::option::Option<f64>,
    /// <p>The date and time, in <a href="http://www.iso.org/iso/iso8601">ISO 8601 date-time format</a>, when the policy was last updated.</p>
    /// <p>When a policy has only one version, this field contains the date and time when the policy was created. When a policy has more than one version, this field contains the date and time when the most recent policy version was created.</p>
    pub update_date: ::std::option::Option<f64>,
    // pub tags: ::std::option::Option<::std::vec::Vec<crate::types::Tag>>,
    pub full_admin_permissions: Option<bool>,
}

impl From<aws_sdk_iam::types::Policy> for Policy {
    fn from(value: aws_sdk_iam::types::Policy) -> Self {
        Self {
            policy_name: value.policy_name,
            policy_id: value.policy_id,
            arn: value.arn,
            path: value.path,
            default_version_id: value.default_version_id,
            attachment_count: value.attachment_count,
            permissions_boundary_usage_count: value.permissions_boundary_usage_count,
            is_attachable: value.is_attachable,
            description: value.description,
            create_date: value.create_date.map(|cd| cd.as_secs_f64()),
            update_date: value.update_date.map(|ud| ud.as_secs_f64()),
            full_admin_permissions: None,
        }
    }
}
