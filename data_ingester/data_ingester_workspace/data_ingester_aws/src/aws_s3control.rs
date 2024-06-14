use serde::Serialize;

use data_ingester_splunk::splunk::ToHecEvents;

#[derive(::std::clone::Clone, ::std::cmp::PartialEq, ::std::fmt::Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetPublicAccessBlockOutput {
    /// <p>The <code>PublicAccessBlock</code> configuration currently in effect for this Amazon S3 bucket.</p>
    pub public_access_block_configuration: ::std::option::Option<PublicAccessBlockConfiguration>,
    pub account_id: Option<String>,
}

impl ToHecEvents for &GetPublicAccessBlockOutput {
    type Item = Self;

    fn source(&self) -> &str {
        "s3control_GetPublicAccessBlock"
    }

    fn sourcetype(&self) -> &str {
        "ssphp:aws:json"
    }

    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        Box::new(std::iter::once(self))
    }
    fn ssphp_run_key(&self) -> &str {
        "aws"
    }
}

impl From<aws_sdk_s3control::operation::get_public_access_block::GetPublicAccessBlockOutput>
    for GetPublicAccessBlockOutput
{
    fn from(
        value: aws_sdk_s3control::operation::get_public_access_block::GetPublicAccessBlockOutput,
    ) -> Self {
        Self {
            public_access_block_configuration: value
                .public_access_block_configuration
                .map(|pabc| pabc.into()),
            account_id: None,
        }
    }
}

#[derive(::std::clone::Clone, ::std::cmp::PartialEq, ::std::fmt::Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicAccessBlockConfiguration {
    /// <p>Specifies whether Amazon S3 should block public access control lists (ACLs) for buckets in this account. Setting this element to <code>TRUE</code> causes the following behavior:</p>
    /// <ul>
    /// <li>
    /// <p><code>PutBucketAcl</code> and <code>PutObjectAcl</code> calls fail if the specified ACL is public.</p></li>
    /// <li>
    /// <p>PUT Object calls fail if the request includes a public ACL.</p></li>
    /// <li>
    /// <p>PUT Bucket calls fail if the request includes a public ACL.</p></li>
    /// </ul>
    /// <p>Enabling this setting doesn't affect existing policies or ACLs.</p>
    /// <p>This property is not supported for Amazon S3 on Outposts.</p>
    pub block_public_acls: ::std::option::Option<bool>,
    /// <p>Specifies whether Amazon S3 should ignore public ACLs for buckets in this account. Setting this element to <code>TRUE</code> causes Amazon S3 to ignore all public ACLs on buckets in this account and any objects that they contain.</p>
    /// <p>Enabling this setting doesn't affect the persistence of any existing ACLs and doesn't prevent new public ACLs from being set.</p>
    /// <p>This property is not supported for Amazon S3 on Outposts.</p>
    pub ignore_public_acls: ::std::option::Option<bool>,
    /// <p>Specifies whether Amazon S3 should block public bucket policies for buckets in this account. Setting this element to <code>TRUE</code> causes Amazon S3 to reject calls to PUT Bucket policy if the specified bucket policy allows public access.</p>
    /// <p>Enabling this setting doesn't affect existing bucket policies.</p>
    /// <p>This property is not supported for Amazon S3 on Outposts.</p>
    pub block_public_policy: ::std::option::Option<bool>,
    /// <p>Specifies whether Amazon S3 should restrict public bucket policies for buckets in this account. Setting this element to <code>TRUE</code> restricts access to buckets with public policies to only Amazon Web Service principals and authorized users within this account.</p>
    /// <p>Enabling this setting doesn't affect previously stored bucket policies, except that public and cross-account access within any public bucket policy, including non-public delegation to specific accounts, is blocked.</p>
    /// <p>This property is not supported for Amazon S3 on Outposts.</p>
    pub restrict_public_buckets: ::std::option::Option<bool>,
}

impl From<aws_sdk_s3control::types::PublicAccessBlockConfiguration>
    for PublicAccessBlockConfiguration
{
    fn from(value: aws_sdk_s3control::types::PublicAccessBlockConfiguration) -> Self {
        Self {
            block_public_acls: value.block_public_acls,
            ignore_public_acls: value.ignore_public_acls,
            block_public_policy: value.block_public_policy,
            restrict_public_buckets: value.restrict_public_buckets,
        }
    }
}

#[derive(::std::clone::Clone, ::std::cmp::PartialEq, ::std::fmt::Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetPublicAccessBlocks {
    pub inner: Vec<GetPublicAccessBlockOutput>,
}
