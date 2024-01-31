// use aws_sdk_cloudtrail::types::AdvancedEventSelector;
// use aws_sdk_cloudtrail::types::AdvancedFieldSelector;
use aws_sdk_cloudtrail::operation::get_trail_status::GetTrailStatusOutput;
use aws_sdk_cloudtrail::types::Trail;
use serde::Serialize;

use data_ingester_splunk::splunk::ToHecEvents;

#[derive(serde::Serialize, Debug)]
pub struct TrailWrappers {
    pub inner: Vec<TrailWrapper>,
}

impl ToHecEvents for &TrailWrappers {
    type Item = TrailWrapper;

    fn source(&self) -> &str {
        "cloudtrail_DescribeTrails"
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
pub struct TrailWrapper {
    #[serde(flatten)]
    #[serde(with = "TrailDef")]
    pub trail: Trail,
    #[serde(with = "GetTrailStatusOutputDef")]
    pub trail_status: GetTrailStatusOutput,
    pub event_selectors: GetEventSelectorsOutputSerde,
}

#[derive(Serialize, Debug)]
#[serde(remote = "Trail")]
#[serde(rename_all = "PascalCase")]
pub struct TrailDef {
    /// <p>Name of the trail set by calling <code>CreateTrail</code>. The maximum length is 128 characters.</p>
    pub name: ::std::option::Option<::std::string::String>,
    /// <p>Name of the Amazon S3 bucket into which CloudTrail delivers your trail files. See <a href="https://docs.aws.amazon.com/awscloudtrail/latest/userguide/create_trail_naming_policy.html">Amazon S3 Bucket Naming Requirements</a>.</p>
    pub s3_bucket_name: ::std::option::Option<::std::string::String>,
    /// <p>Specifies the Amazon S3 key prefix that comes after the name of the bucket you have designated for log file delivery. For more information, see <a href="https://docs.aws.amazon.com/awscloudtrail/latest/userguide/cloudtrail-find-log-files.html">Finding Your CloudTrail Log Files</a>. The maximum length is 200 characters.</p>
    pub s3_key_prefix: ::std::option::Option<::std::string::String>,
    /// <p>This field is no longer in use. Use <code>SnsTopicARN</code>.</p>
    #[deprecated]
    pub sns_topic_name: ::std::option::Option<::std::string::String>,
    /// <p>Specifies the ARN of the Amazon SNS topic that CloudTrail uses to send notifications when log files are delivered. The following is the format of a topic ARN.</p>
    /// <p><code>arn:aws:sns:us-east-2:123456789012:MyTopic</code></p>
    pub sns_topic_arn: ::std::option::Option<::std::string::String>,
    /// <p>Set to <b>True</b> to include Amazon Web Services API calls from Amazon Web Services global services such as IAM. Otherwise, <b>False</b>.</p>
    pub include_global_service_events: ::std::option::Option<bool>,
    /// <p>Specifies whether the trail exists only in one Region or exists in all Regions.</p>
    pub is_multi_region_trail: ::std::option::Option<bool>,
    /// <p>The Region in which the trail was created.</p>
    pub home_region: ::std::option::Option<::std::string::String>,
    /// <p>Specifies the ARN of the trail. The following is the format of a trail ARN.</p>
    /// <p><code>arn:aws:cloudtrail:us-east-2:123456789012:trail/MyTrail</code></p>
    pub trail_arn: ::std::option::Option<::std::string::String>,
    /// <p>Specifies whether log file validation is enabled.</p>
    pub log_file_validation_enabled: ::std::option::Option<bool>,
    /// <p>Specifies an Amazon Resource Name (ARN), a unique identifier that represents the log group to which CloudTrail logs will be delivered.</p>
    pub cloud_watch_logs_log_group_arn: ::std::option::Option<::std::string::String>,
    /// <p>Specifies the role for the CloudWatch Logs endpoint to assume to write to a user's log group.</p>
    pub cloud_watch_logs_role_arn: ::std::option::Option<::std::string::String>,
    /// <p>Specifies the KMS key ID that encrypts the logs delivered by CloudTrail. The value is a fully specified ARN to a KMS key in the following format.</p>
    /// <p><code>arn:aws:kms:us-east-2:123456789012:key/12345678-1234-1234-1234-123456789012</code></p>
    pub kms_key_id: ::std::option::Option<::std::string::String>,
    /// <p>Specifies if the trail has custom event selectors.</p>
    pub has_custom_event_selectors: ::std::option::Option<bool>,
    /// <p>Specifies whether a trail has insight types specified in an <code>InsightSelector</code> list.</p>
    pub has_insight_selectors: ::std::option::Option<bool>,
    /// <p>Specifies whether the trail is an organization trail.</p>
    pub is_organization_trail: ::std::option::Option<bool>,
}

#[derive(Serialize)]
#[serde(remote = "GetTrailStatusOutput")]
#[serde(rename_all = "PascalCase")]
pub struct GetTrailStatusOutputDef {
    /// <p>Whether the CloudTrail trail is currently logging Amazon Web Services API calls.</p>
    pub is_logging: ::std::option::Option<bool>,
    /// <p>Displays any Amazon S3 error that CloudTrail encountered when attempting to deliver log files to the designated bucket. For more information, see <a href="https://docs.aws.amazon.com/AmazonS3/latest/API/ErrorResponses.html">Error Responses</a> in the Amazon S3 API Reference.</p><note>
    /// <p>This error occurs only when there is a problem with the destination S3 bucket, and does not occur for requests that time out. To resolve the issue, create a new bucket, and then call <code>UpdateTrail</code> to specify the new bucket; or fix the existing objects so that CloudTrail can again write to the bucket.</p>
    /// </note>
    pub latest_delivery_error: ::std::option::Option<::std::string::String>,
    /// <p>Displays any Amazon SNS error that CloudTrail encountered when attempting to send a notification. For more information about Amazon SNS errors, see the <a href="https://docs.aws.amazon.com/sns/latest/dg/welcome.html">Amazon SNS Developer Guide</a>.</p>
    pub latest_notification_error: ::std::option::Option<::std::string::String>,
    /// <p>Specifies the date and time that CloudTrail last delivered log files to an account's Amazon S3 bucket.</p>
    #[serde(with = "date_time_def")]
    pub latest_delivery_time: ::std::option::Option<::aws_smithy_types::DateTime>,
    /// <p>Specifies the date and time of the most recent Amazon SNS notification that CloudTrail has written a new log file to an account's Amazon S3 bucket.</p>
    #[serde(with = "date_time_def")]
    pub latest_notification_time: ::std::option::Option<::aws_smithy_types::DateTime>,
    /// <p>Specifies the most recent date and time when CloudTrail started recording API calls for an Amazon Web Services account.</p>
    #[serde(with = "date_time_def")]
    pub start_logging_time: ::std::option::Option<::aws_smithy_types::DateTime>,
    /// <p>Specifies the most recent date and time when CloudTrail stopped recording API calls for an Amazon Web Services account.</p>
    #[serde(with = "date_time_def")]
    pub stop_logging_time: ::std::option::Option<::aws_smithy_types::DateTime>,
    /// <p>Displays any CloudWatch Logs error that CloudTrail encountered when attempting to deliver logs to CloudWatch Logs.</p>
    pub latest_cloud_watch_logs_delivery_error: ::std::option::Option<::std::string::String>,
    /// <p>Displays the most recent date and time when CloudTrail delivered logs to CloudWatch Logs.</p>
    #[serde(with = "date_time_def")]
    pub latest_cloud_watch_logs_delivery_time: ::std::option::Option<::aws_smithy_types::DateTime>,
    /// <p>Specifies the date and time that CloudTrail last delivered a digest file to an account's Amazon S3 bucket.</p>
    #[serde(with = "date_time_def")]
    pub latest_digest_delivery_time: ::std::option::Option<::aws_smithy_types::DateTime>,
    /// <p>Displays any Amazon S3 error that CloudTrail encountered when attempting to deliver a digest file to the designated bucket. For more information, see <a href="https://docs.aws.amazon.com/AmazonS3/latest/API/ErrorResponses.html">Error Responses</a> in the Amazon S3 API Reference.</p><note>
    /// <p>This error occurs only when there is a problem with the destination S3 bucket, and does not occur for requests that time out. To resolve the issue, create a new bucket, and then call <code>UpdateTrail</code> to specify the new bucket; or fix the existing objects so that CloudTrail can again write to the bucket.</p>
    /// </note>
    pub latest_digest_delivery_error: ::std::option::Option<::std::string::String>,
    /// <p>This field is no longer in use.</p>
    pub latest_delivery_attempt_time: ::std::option::Option<::std::string::String>,
    /// <p>This field is no longer in use.</p>
    pub latest_notification_attempt_time: ::std::option::Option<::std::string::String>,
    /// <p>This field is no longer in use.</p>
    pub latest_notification_attempt_succeeded: ::std::option::Option<::std::string::String>,
    /// <p>This field is no longer in use.</p>
    pub latest_delivery_attempt_succeeded: ::std::option::Option<::std::string::String>,
    /// <p>This field is no longer in use.</p>
    pub time_logging_started: ::std::option::Option<::std::string::String>,
    /// <p>This field is no longer in use.</p>
    pub time_logging_stopped: ::std::option::Option<::std::string::String>,
}

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct GetEventSelectorsOutputSerde {
    /// <p>The specified trail ARN that has the event selectors.</p>
    #[serde(skip_serializing)]
    pub trail_arn: ::std::option::Option<::std::string::String>,
    /// <p>The event selectors that are configured for the trail.</p>
    pub event_selectors: ::std::option::Option<::std::vec::Vec<EventSelector>>,
    // <p>The advanced event selectors that are configured for the trail.</p>
    pub advanced_event_selectors: ::std::option::Option<::std::vec::Vec<AdvancedEventSelector>>,
}

impl From<aws_sdk_cloudtrail::operation::get_event_selectors::GetEventSelectorsOutput>
    for GetEventSelectorsOutputSerde
{
    fn from(
        value: aws_sdk_cloudtrail::operation::get_event_selectors::GetEventSelectorsOutput,
    ) -> Self {
        Self {
            trail_arn: value.trail_arn,
            event_selectors: value
                .event_selectors
                .map(|vec| vec.into_iter().map(|es| es.into()).collect()),
            advanced_event_selectors: value
                .advanced_event_selectors
                .map(|vec| vec.into_iter().map(|aes| aes.into()).collect()),
        }
    }
}

#[derive(::std::clone::Clone, ::std::cmp::PartialEq, ::std::fmt::Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct EventSelector {
    /// <p>Specify if you want your trail to log read-only events, write-only events, or all. For example, the EC2 <code>GetConsoleOutput</code> is a read-only API operation and <code>RunInstances</code> is a write-only API operation.</p>
    /// <p>By default, the value is <code>All</code>.</p>
    pub read_write_type: Option<String>,
    /// <p>Specify if you want your event selector to include management events for your trail.</p>
    /// <p>For more information, see <a href="https://docs.aws.amazon.com/awscloudtrail/latest/userguide/logging-management-events-with-cloudtrail.html">Management Events</a> in the <i>CloudTrail User Guide</i>.</p>
    /// <p>By default, the value is <code>true</code>.</p>
    /// <p>The first copy of management events is free. You are charged for additional copies of management events that you are logging on any subsequent trail in the same Region. For more information about CloudTrail pricing, see <a href="http://aws.amazon.com/cloudtrail/pricing/">CloudTrail Pricing</a>.</p>
    pub include_management_events: ::std::option::Option<bool>,
    /// <p>CloudTrail supports data event logging for Amazon S3 objects, Lambda functions, and Amazon DynamoDB tables with basic event selectors. You can specify up to 250 resources for an individual event selector, but the total number of data resources cannot exceed 250 across all event selectors in a trail. This limit does not apply if you configure resource logging for all data events.</p>
    /// <p>For more information, see <a href="https://docs.aws.amazon.com/awscloudtrail/latest/userguide/logging-data-events-with-cloudtrail.html">Data Events</a> and <a href="https://docs.aws.amazon.com/awscloudtrail/latest/userguide/WhatIsCloudTrail-Limits.html">Limits in CloudTrail</a> in the <i>CloudTrail User Guide</i>.</p>
    pub data_resources: ::std::option::Option<Vec<DataResource>>,
    /// <p>An optional list of service event sources from which you do not want management events to be logged on your trail. In this release, the list can be empty (disables the filter), or it can filter out Key Management Service or Amazon RDS Data API events by containing <code>kms.amazonaws.com</code> or <code>rdsdata.amazonaws.com</code>. By default, <code>ExcludeManagementEventSources</code> is empty, and KMS and Amazon RDS Data API events are logged to your trail. You can exclude management event sources only in Regions that support the event source.</p>
    pub exclude_management_event_sources:
        ::std::option::Option<::std::vec::Vec<::std::string::String>>,
}

impl From<aws_sdk_cloudtrail::types::EventSelector> for EventSelector {
    fn from(value: aws_sdk_cloudtrail::types::EventSelector) -> Self {
        Self {
            read_write_type: value.read_write_type.map(|rwt| rwt.as_str().to_owned()),
            include_management_events: value.include_management_events,
            data_resources: value
                .data_resources
                .map(|vec| vec.into_iter().map(|dr| dr.into()).collect()),
            exclude_management_event_sources: value.exclude_management_event_sources,
        }
    }
}

#[derive(::std::clone::Clone, ::std::cmp::PartialEq, ::std::fmt::Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct DataResource {
    pub r#type: ::std::option::Option<::std::string::String>,
    pub values: ::std::option::Option<::std::vec::Vec<::std::string::String>>,
}

impl From<aws_sdk_cloudtrail::types::DataResource> for DataResource {
    fn from(value: aws_sdk_cloudtrail::types::DataResource) -> Self {
        Self {
            r#type: value.r#type,
            values: value.values,
        }
    }
}

#[derive(::std::clone::Clone, ::std::cmp::PartialEq, ::std::fmt::Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct AdvancedEventSelector {
    /// <p>An optional, descriptive name for an advanced event selector, such as "Log data events for only two S3 buckets".</p>
    pub name: ::std::option::Option<::std::string::String>,
    /// <p>Contains all selector statements in an advanced event selector.</p>
    pub field_selectors: ::std::vec::Vec<AdvancedFieldSelector>,
}

impl From<aws_sdk_cloudtrail::types::AdvancedEventSelector> for AdvancedEventSelector {
    fn from(value: aws_sdk_cloudtrail::types::AdvancedEventSelector) -> Self {
        Self {
            name: value.name,
            field_selectors: value
                .field_selectors
                .into_iter()
                .map(|fs| fs.into())
                .collect(),
        }
    }
}

#[derive(::std::clone::Clone, ::std::cmp::PartialEq, ::std::fmt::Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct AdvancedFieldSelector {
    pub field: ::std::string::String,
    /// <p>An operator that includes events that match the exact value of the event record field specified as the value of <code>Field</code>. This is the only valid operator that you can use with the <code>readOnly</code>, <code>eventCategory</code>, and <code>resources.type</code> fields.</p>
    pub equals: ::std::option::Option<::std::vec::Vec<::std::string::String>>,
    /// <p>An operator that includes events that match the first few characters of the event record field specified as the value of <code>Field</code>.</p>
    pub starts_with: ::std::option::Option<::std::vec::Vec<::std::string::String>>,
    /// <p>An operator that includes events that match the last few characters of the event record field specified as the value of <code>Field</code>.</p>
    pub ends_with: ::std::option::Option<::std::vec::Vec<::std::string::String>>,
    /// <p>An operator that excludes events that match the exact value of the event record field specified as the value of <code>Field</code>.</p>
    pub not_equals: ::std::option::Option<::std::vec::Vec<::std::string::String>>,
    /// <p>An operator that excludes events that match the first few characters of the event record field specified as the value of <code>Field</code>.</p>
    pub not_starts_with: ::std::option::Option<::std::vec::Vec<::std::string::String>>,
    /// <p>An operator that excludes events that match the last few characters of the event record field specified as the value of <code>Field</code>.</p>
    pub not_ends_with: ::std::option::Option<::std::vec::Vec<::std::string::String>>,
}

impl From<aws_sdk_cloudtrail::types::AdvancedFieldSelector> for AdvancedFieldSelector {
    fn from(value: aws_sdk_cloudtrail::types::AdvancedFieldSelector) -> Self {
        Self {
            field: value.field,
            equals: value.equals,
            starts_with: value.starts_with,
            ends_with: value.ends_with,
            not_equals: value.not_equals,
            not_starts_with: value.not_starts_with,
            not_ends_with: value.not_ends_with,
        }
    }
}

pub mod date_time_def {
    use aws_smithy_types::DateTime;
    use serde::{Serialize, Serializer};

    pub fn serialize<S>(value: &Option<DateTime>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        value
            .as_ref()
            .map(|dt| dt.as_secs_f64())
            .serialize(serializer)
    }
}
