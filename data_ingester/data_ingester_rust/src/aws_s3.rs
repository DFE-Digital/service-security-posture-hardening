use serde::Serialize;

use crate::splunk::ToHecEvents;

#[derive(serde::Serialize, Debug)]
pub struct GetBucketAclOutputs {
    pub inner: Vec<GetBucketAclOutput>,
}

impl ToHecEvents for &GetBucketAclOutputs {
    type Item = GetBucketAclOutput;

    fn source(&self) -> &str {
        "s3_GetBucketAcl"
    }

    fn sourcetype(&self) -> &str {
        "ssphp:aws:json"
    }

    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        Box::new(self.inner.iter())
    }
}

#[derive(serde::Serialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct GetBucketAclOutput {
    /// <p>Container for the bucket owner's display name and ID.</p>
    pub owner: ::std::option::Option<Owner>,
    /// <p>A list of grants.</p>
    pub grants: ::std::option::Option<::std::vec::Vec<Grant>>,
    pub trail_arn: Option<String>,
    pub bucket_name: Option<String>,
}

impl From<aws_sdk_s3::operation::get_bucket_acl::GetBucketAclOutput> for GetBucketAclOutput {
    fn from(value: aws_sdk_s3::operation::get_bucket_acl::GetBucketAclOutput) -> Self {
        Self {
            owner: value.owner.map(|o| o.into()),
            grants: value
                .grants
                .map(|vec| vec.into_iter().map(|g| g.into()).collect()),
            trail_arn: None,
            bucket_name: None,
        }
    }
}

#[derive(serde::Serialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct Owner {
    pub display_name: ::std::option::Option<::std::string::String>,
    pub id: ::std::option::Option<::std::string::String>,
}

impl From<aws_sdk_s3::types::Owner> for Owner {
    fn from(value: aws_sdk_s3::types::Owner) -> Self {
        Self {
            display_name: value.display_name,
            id: value.id,
        }
    }
}

#[derive(serde::Serialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct Grant {
    /// <p>The person being granted permissions.</p>
    pub grantee: ::std::option::Option<Grantee>,
    /// <p>Specifies the permission given to the grantee.</p>
    pub permission: ::std::option::Option<String>,
}

impl From<aws_sdk_s3::types::Grant> for Grant {
    fn from(value: aws_sdk_s3::types::Grant) -> Self {
        Self {
            grantee: value.grantee.map(|g| g.into()),
            permission: value.permission.map(|p| p.as_str().to_owned()),
        }
    }
}

#[derive(::std::clone::Clone, ::std::cmp::PartialEq, ::std::fmt::Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct Grantee {
    /// <p>Screen name of the grantee.</p>
    pub display_name: ::std::option::Option<::std::string::String>,
    /// <p>Email address of the grantee.</p><note>
    pub email_address: ::std::option::Option<::std::string::String>,
    /// <p>The canonical user ID of the grantee.</p>
    pub id: ::std::option::Option<::std::string::String>,
    /// <p>URI of the grantee group.</p>
    pub uri: ::std::option::Option<::std::string::String>,
    /// <p>Type of grantee</p>
    pub r#type: String,
}

impl From<aws_sdk_s3::types::Grantee> for Grantee {
    fn from(value: aws_sdk_s3::types::Grantee) -> Self {
        Self {
            display_name: value.display_name,
            email_address: value.email_address,
            id: value.id,
            uri: value.uri,
            r#type: value.r#type.as_str().to_owned(),
        }
    }
}

#[derive(serde::Serialize, Debug)]
pub struct GetBucketPolicyOutputs {
    pub inner: Vec<GetBucketPolicyOutput>,
}

impl ToHecEvents for &GetBucketPolicyOutputs {
    type Item = GetBucketPolicyOutput;

    fn source(&self) -> &str {
        "s3_GetBucketPolicy"
    }

    fn sourcetype(&self) -> &str {
        "ssphp:aws:json"
    }

    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        Box::new(self.inner.iter())
    }
}

#[derive(serde::Serialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct GetBucketPolicyOutput {
    /// <p>The bucket policy as a JSON document.</p>
    #[serde(flatten)]
    pub policy: Option<serde_json::Value>,
    pub bucket_name: Option<String>,
    pub trail_arn: Option<String>,
}

impl ToHecEvents for &GetBucketPolicyOutput {
    type Item = Self;

    fn source(&self) -> &str {
        "s3_GetBucketPolicy"
    }

    fn sourcetype(&self) -> &str {
        "ssphp:aws:json"
    }

    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        Box::new(std::iter::once(self))
    }
}

impl From<aws_sdk_s3::operation::get_bucket_policy::GetBucketPolicyOutput>
    for GetBucketPolicyOutput
{
    fn from(value: aws_sdk_s3::operation::get_bucket_policy::GetBucketPolicyOutput) -> Self {
        Self {
            policy: value.policy().map(|p| serde_json::from_str(p).unwrap()),
            bucket_name: None,
            trail_arn: None,
        }
    }
}

#[derive(serde::Serialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct GetBucketLoggingOutputs {
    pub inner: Vec<GetBucketLoggingOutput>,
}

impl ToHecEvents for &GetBucketLoggingOutputs {
    type Item = GetBucketLoggingOutput;

    fn source(&self) -> &str {
        "s3_GetBucketLogging"
    }

    fn sourcetype(&self) -> &str {
        "ssphp:aws:json"
    }

    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        Box::new(self.inner.iter())
    }
}

#[derive(::std::clone::Clone, ::std::cmp::PartialEq, ::std::fmt::Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct GetBucketLoggingOutput {
    /// <p>Describes where logs are stored and the prefix that Amazon S3 assigns to all log object keys for a bucket. For more information, see <a href="https://docs.aws.amazon.com/AmazonS3/latest/API/RESTBucketPUTlogging.html">PUT Bucket logging</a> in the <i>Amazon S3 API Reference</i>.</p>
    pub logging_enabled: ::std::option::Option<LoggingEnabled>,
    pub bucket_name: Option<String>,
    pub trail_arn: Option<String>,
}

impl From<aws_sdk_s3::operation::get_bucket_logging::GetBucketLoggingOutput>
    for GetBucketLoggingOutput
{
    fn from(value: aws_sdk_s3::operation::get_bucket_logging::GetBucketLoggingOutput) -> Self {
        Self {
            logging_enabled: value.logging_enabled.map(|le| le.into()),
            bucket_name: None,
            trail_arn: None,
        }
    }
}

#[derive(::std::clone::Clone, ::std::cmp::PartialEq, ::std::fmt::Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct LoggingEnabled {
    /// <p>Specifies the bucket where you want Amazon S3 to store server access logs. You can have your logs delivered to any bucket that you own, including the same bucket that is being logged. You can also configure multiple buckets to deliver their logs to the same target bucket. In this case, you should choose a different <code>TargetPrefix</code> for each source bucket so that the delivered log files can be distinguished by key.</p>
    pub target_bucket: ::std::string::String,
    /// <p>Container for granting information.</p>
    /// <p>Buckets that use the bucket owner enforced setting for Object Ownership don't support target grants. For more information, see <a href="https://docs.aws.amazon.com/AmazonS3/latest/userguide/enable-server-access-logging.html#grant-log-delivery-permissions-general">Permissions for server access log delivery</a> in the <i>Amazon S3 User Guide</i>.</p>
    pub target_grants: ::std::option::Option<::std::vec::Vec<TargetGrant>>,
    /// <p>A prefix for all log object keys. If you store log files from multiple Amazon S3 buckets in a single bucket, you can use a prefix to distinguish which log files came from which bucket.</p>
    pub target_prefix: ::std::string::String,
    /// <p>Amazon S3 key format for log objects.</p>
    pub target_object_key_format: ::std::option::Option<TargetObjectKeyFormat>,
}

impl From<aws_sdk_s3::types::LoggingEnabled> for LoggingEnabled {
    fn from(value: aws_sdk_s3::types::LoggingEnabled) -> Self {
        Self {
            target_bucket: value.target_bucket,
            target_grants: value
                .target_grants
                .map(|vec| vec.into_iter().map(|g| g.into()).collect()),
            target_prefix: value.target_prefix,
            target_object_key_format: value.target_object_key_format.map(|tokf| tokf.into()),
        }
    }
}

#[derive(::std::clone::Clone, ::std::cmp::PartialEq, ::std::fmt::Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct TargetGrant {
    /// <p>Container for the person being granted permissions.</p>
    pub grantee: ::std::option::Option<Grantee>,
    /// <p>Logging permissions assigned to the grantee for the bucket.</p>
    pub permission: ::std::option::Option<String>,
}

impl From<aws_sdk_s3::types::TargetGrant> for TargetGrant {
    fn from(value: aws_sdk_s3::types::TargetGrant) -> Self {
        Self {
            grantee: value.grantee.map(|g| g.into()),
            permission: value.permission.map(|p| p.as_str().to_owned()),
        }
    }
}

#[derive(::std::clone::Clone, ::std::cmp::PartialEq, ::std::fmt::Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct TargetObjectKeyFormat {
    /// <p>To use the simple format for S3 keys for log objects. To specify SimplePrefix format, set SimplePrefix to {}.</p>
    pub simple_prefix: ::std::option::Option<SimplePrefix>,
    /// <p>Partitioned S3 key for log objects.</p>
    pub partitioned_prefix: ::std::option::Option<PartitionedPrefix>,
}

impl From<aws_sdk_s3::types::TargetObjectKeyFormat> for TargetObjectKeyFormat {
    fn from(value: aws_sdk_s3::types::TargetObjectKeyFormat) -> Self {
        Self {
            simple_prefix: value.simple_prefix.map(|sp| sp.into()),
            partitioned_prefix: value.partitioned_prefix.map(|pp| pp.into()),
        }
    }
}

#[derive(::std::clone::Clone, ::std::cmp::PartialEq, ::std::fmt::Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct SimplePrefix {}

impl From<aws_sdk_s3::types::SimplePrefix> for SimplePrefix {
    fn from(_value: aws_sdk_s3::types::SimplePrefix) -> Self {
        Self {}
    }
}

#[derive(::std::clone::Clone, ::std::cmp::PartialEq, ::std::fmt::Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct PartitionedPrefix {
    /// <p>Specifies the partition date source for the partitioned prefix. PartitionDateSource can be EventTime or DeliveryTime.</p>
    pub partition_date_source: ::std::option::Option<String>,
}

impl From<aws_sdk_s3::types::PartitionedPrefix> for PartitionedPrefix {
    fn from(value: aws_sdk_s3::types::PartitionedPrefix) -> Self {
        Self {
            partition_date_source: value
                .partition_date_source
                .map(|pds| pds.as_str().to_owned()),
        }
    }
}
