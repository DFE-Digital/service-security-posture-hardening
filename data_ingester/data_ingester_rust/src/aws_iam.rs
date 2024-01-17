use serde::Serialize;

use crate::aws_trail::date_time_def;
use crate::splunk::ToHecEvents;

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
    pub user: ::std::option::Option<crate::aws::User>,
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
}
