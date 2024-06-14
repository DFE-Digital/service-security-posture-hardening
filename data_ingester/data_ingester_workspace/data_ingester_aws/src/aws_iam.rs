use serde::{Deserialize, Serialize};

use crate::aws_trail::date_time_def;
use data_ingester_splunk::splunk::ToHecEvents;

#[derive(::std::clone::Clone, ::std::cmp::PartialEq, Serialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct VirtualMfaDevice {
    /// <p>The serial number associated with <code>VirtualMFADevice</code>.</p>
    pub serial_number: ::std::string::String,
    /// <p>The base32 seed defined as specified in <a href="https://tools.ietf.org/html/rfc3548.txt">RFC3548</a>. The <code>Base32StringSeed</code> is base32-encoded.</p>
    //pub base32_string_seed: ::std::option::Option<::aws_smithy_types::Blob>,
    /// <p>A QR code PNG image that encodes <code>otpauth://totp/$virtualMFADeviceName@$AccountName?secret=$Base32String</code> where <code>$virtualMFADeviceName</code> is one of the create call arguments. <code>AccountName</code> is the user name if set (otherwise, the account ID otherwise), and <code>Base32String</code> is the seed in base32 format. The <code>Base32String</code> value is base64-encoded.</p>
    // pub qr_code_png: ::std::option::Option<::aws_smithy_types::Blob>,
    /// <p>The IAM user associated with this virtual MFA device.</p>
    pub user: ::std::option::Option<crate::aws_iam::User>,
    /// <p>The date and time on which the virtual MFA device was enabled.</p>
    #[serde(with = "date_time_def")]
    pub enable_date: ::std::option::Option<::aws_smithy_types::DateTime>,
    // <p>A list of tags that are attached to the virtual MFA device. For more information about tagging, see <a href="https://docs.aws.amazon.com/IAM/latest/UserGuide/id_tags.html">Tagging IAM resources</a> in the <i>IAM User Guide</i>.</p>
}

impl From<aws_sdk_iam::types::VirtualMfaDevice> for VirtualMfaDevice {
    fn from(value: aws_sdk_iam::types::VirtualMfaDevice) -> Self {
        Self {
            serial_number: value.serial_number,
            user: value.user.map(|u| u.into()),
            enable_date: value.enable_date,
        }
    }
}

#[derive(::std::clone::Clone, ::std::cmp::PartialEq, Serialize, Debug, Default)]
pub struct VirtualMfaDevices {
    pub inner: Vec<VirtualMfaDevice>,
}

impl ToHecEvents for &VirtualMfaDevices {
    type Item = VirtualMfaDevice;

    fn source(&self) -> &str {
        "iam_ListVirtualMfaDevices"
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

#[derive(::std::clone::Clone, ::std::cmp::PartialEq, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct MfaDevice {
    /// <p>The serial number that uniquely identifies the MFA device. For virtual MFA devices, the serial number is the device ARN.</p>
    pub serial_number: String,
    /// <p>The user with whom the MFA device is associated.</p>
    pub user_name: String,
    /// <p>The date when the MFA device was enabled for the user.</p>
    #[serde(with = "date_time_def")]
    pub enable_date: Option<aws_smithy_types::DateTime>,
}

impl From<aws_sdk_iam::types::MfaDevice> for MfaDevice {
    fn from(value: aws_sdk_iam::types::MfaDevice) -> Self {
        Self {
            serial_number: value.serial_number,
            user_name: value.user_name,
            enable_date: Some(value.enable_date),
        }
    }
}

#[derive(::std::clone::Clone, ::std::cmp::PartialEq, Serialize, Debug, Default)]
pub struct MfaDevices {
    pub inner: Vec<MfaDevice>,
}

impl ToHecEvents for &MfaDevices {
    type Item = MfaDevice;

    fn source(&self) -> &str {
        "iam_ListMfaDevices"
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

#[derive(::std::clone::Clone, ::std::fmt::Debug, Serialize)]
pub struct Group {
    /// <p>The path to the group. For more information about paths, see <a href="https://docs.aws.amazon.com/IAM/latest/UserGuide/Using_Identifiers.html">IAM identifiers</a> in the <i>IAM User Guide</i>.</p>
    pub path: ::std::string::String,
    /// <p>The friendly name that identifies the group.</p>
    pub group_name: ::std::string::String,
    /// <p>The stable and unique string identifying the group. For more information about IDs, see <a href="https://docs.aws.amazon.com/IAM/latest/UserGuide/Using_Identifiers.html">IAM identifiers</a> in the <i>IAM User Guide</i>.</p>
    pub group_id: ::std::string::String,
    /// <p>The Amazon Resource Name (ARN) specifying the group. For more information about ARNs and how to use them in policies, see <a href="https://docs.aws.amazon.com/IAM/latest/UserGuide/Using_Identifiers.html">IAM identifiers</a> in the <i>IAM User Guide</i>.</p>
    pub arn: ::std::string::String,
    /// <p>The date and time, in <a href="http://www.iso.org/iso/iso8601">ISO 8601 date-time format</a>, when the group was created.</p>
    #[serde(with = "date_time_def")]
    pub create_date: Option<::aws_smithy_types::DateTime>,
    pub(crate) attached_policies: Vec<AttachedPolicy>,
    pub(crate) policies: Vec<String>,
    pub(crate) users: Vec<String>,
}

impl From<aws_sdk_iam::types::Group> for Group {
    fn from(value: aws_sdk_iam::types::Group) -> Self {
        Self {
            path: value.path,
            group_name: value.group_name,
            group_id: value.group_id,
            arn: value.arn,
            create_date: Some(value.create_date),
            attached_policies: vec![],
            policies: vec![],
            users: vec![],
        }
    }
}

#[derive(Default, Debug, Clone, Serialize)]
pub(crate) struct Groups {
    pub(crate) inner: Vec<Group>,
}

impl ToHecEvents for &Groups {
    type Item = Group;

    fn source(&self) -> &str {
        "iam_ListGroups"
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

#[derive(Default, Debug, Clone, Serialize)]
pub(crate) struct Users {
    pub(crate) inner: Vec<User>,
}

impl From<Vec<aws_sdk_iam::types::User>> for Users {
    fn from(value: Vec<aws_sdk_iam::types::User>) -> Self {
        Self {
            inner: value.into_iter().map(|u| u.into()).collect(),
        }
    }
}

impl ToHecEvents for &Users {
    type Item = User;

    fn source(&self) -> &str {
        "iam_ListUsers"
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
pub struct User {
    arn: String,
    path: String,
    pub(crate) user_name: String,
    user_id: String,
    create_date: f64,
    pub(crate) attached_policies: Vec<AttachedPolicy>,
    pub(crate) policies: Vec<String>,
    // password_last_used: Option<f64>,
    // pub password_last_used: ::std::option::Option<::aws_smithy_types::DateTime>,
    // pub permissions_boundary: ::std::option::Option<crate::types::AttachedPermissionsBoundary>,
    // pub tags: ::std::option::Option<::std::vec::Vec<crate::types::Tag>>,
}

impl From<aws_sdk_iam::types::User> for User {
    fn from(value: aws_sdk_iam::types::User) -> Self {
        Self {
            arn: value.arn,
            path: value.path,
            user_name: value.user_name,
            user_id: value.user_id,
            create_date: value.create_date.as_secs_f64(),
            attached_policies: vec![],
            policies: vec![],
        }
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct AttachedPolicy {
    policy_name: Option<String>,
    policy_arn: Option<String>,
}

impl From<aws_sdk_iam::types::AttachedPolicy> for AttachedPolicy {
    fn from(value: aws_sdk_iam::types::AttachedPolicy) -> Self {
        Self {
            policy_arn: value.policy_arn,
            policy_name: value.policy_name,
        }
    }
}
