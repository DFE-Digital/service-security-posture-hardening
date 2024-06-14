use std::iter;

use serde::{Deserialize, Serialize};

use data_ingester_splunk::splunk::ToHecEvents;
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct EntitiesForPolicyOutput {
    pub arn: Option<String>,
    policy_groups: Option<Vec<PolicyGroup>>,
    policy_users: Option<Vec<PolicyUser>>,
    policy_roles: Option<Vec<PolicyRole>>,
}

impl From<aws_sdk_iam::operation::list_entities_for_policy::ListEntitiesForPolicyOutput>
    for EntitiesForPolicyOutput
{
    fn from(
        value: aws_sdk_iam::operation::list_entities_for_policy::ListEntitiesForPolicyOutput,
    ) -> Self {
        Self {
            arn: None,
            policy_groups: value
                .policy_groups
                .map(|groups| groups.into_iter().map(|group| group.into()).collect()),
            policy_users: value
                .policy_users
                .map(|users| users.into_iter().map(|group| group.into()).collect()),
            policy_roles: value
                .policy_roles
                .map(|roles| roles.into_iter().map(|group| group.into()).collect()),
        }
    }
}

impl ToHecEvents for &EntitiesForPolicyOutput {
    type Item = Self;

    fn source(&self) -> &str {
        "iam_EntitiesForPolicyOutput"
    }

    fn sourcetype(&self) -> &str {
        "ssphp:aws:json"
    }

    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        Box::new(iter::once(self))
    }
    fn ssphp_run_key(&self) -> &str {
        "aws"
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct PolicyGroup {
    group_name: Option<String>,
    group_id: Option<String>,
}

impl From<aws_sdk_iam::types::PolicyGroup> for PolicyGroup {
    fn from(value: aws_sdk_iam::types::PolicyGroup) -> Self {
        Self {
            group_name: value.group_name,
            group_id: value.group_id,
        }
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct PolicyUser {
    user_name: Option<String>,
    user_id: Option<String>,
}

impl From<aws_sdk_iam::types::PolicyUser> for PolicyUser {
    fn from(value: aws_sdk_iam::types::PolicyUser) -> Self {
        Self {
            user_name: value.user_name,
            user_id: value.user_id,
        }
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct PolicyRole {
    role_name: Option<String>,
    role_id: Option<String>,
}

impl From<aws_sdk_iam::types::PolicyRole> for PolicyRole {
    fn from(value: aws_sdk_iam::types::PolicyRole) -> Self {
        Self {
            role_name: value.role_name,
            role_id: value.role_id,
        }
    }
}
