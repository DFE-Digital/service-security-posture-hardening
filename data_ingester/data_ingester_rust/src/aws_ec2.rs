use serde::Serialize;

use crate::aws_trail::date_time_def;
use crate::splunk::ToHecEvents;

#[derive(::std::clone::Clone, ::std::cmp::PartialEq, ::std::fmt::Debug, Serialize)]
pub struct DescribeVpcs {
    pub inner: Vec<Vpc>,
}

impl ToHecEvents for &DescribeVpcs {
    type Item = Vpc;

    fn source(&self) -> &str {
        "ec2_DescribeVpcs"
    }

    fn sourcetype(&self) -> &str {
        "ssphp:aws:json"
    }

    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        Box::new(self.inner.iter())
    }
}

#[derive(::std::clone::Clone, ::std::cmp::PartialEq, ::std::fmt::Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Vpc {
    /// <p>The primary IPv4 CIDR block for the VPC.</p>
    pub cidr_block: ::std::option::Option<::std::string::String>,
    /// <p>The ID of the set of DHCP options you've associated with the VPC.</p>
    pub dhcp_options_id: ::std::option::Option<::std::string::String>,
    /// <p>The current state of the VPC.</p>
    pub state: ::std::option::Option<String>,
    /// <p>The ID of the VPC.</p>
    pub vpc_id: ::std::option::Option<::std::string::String>,
    /// <p>The ID of the Amazon Web Services account that owns the VPC.</p>
    pub owner_id: ::std::option::Option<::std::string::String>,
    /// <p>The allowed tenancy of instances launched into the VPC.</p>
    pub instance_tenancy: ::std::option::Option<String>,
    /// <p>Information about the IPv6 CIDR blocks associated with the VPC.</p>
    pub ipv6_cidr_block_association_set:
        ::std::option::Option<::std::vec::Vec<VpcIpv6CidrBlockAssociation>>,
    /// <p>Information about the IPv4 CIDR blocks associated with the VPC.</p>
    pub cidr_block_association_set: ::std::option::Option<::std::vec::Vec<VpcCidrBlockAssociation>>,
    /// <p>Indicates whether the VPC is the default VPC.</p>
    pub is_default: ::std::option::Option<bool>,
    // <p>Any tags assigned to the VPC.</p>
    // pub tags: ::std::option::Option<::std::vec::Vec<crate::types::Tag>>,
}

impl From<aws_sdk_ec2::types::Vpc> for Vpc {
    fn from(value: aws_sdk_ec2::types::Vpc) -> Self {
        Self {
            cidr_block: value.cidr_block,
            dhcp_options_id: value.dhcp_options_id,
            state: value.state.map(|s| s.as_str().to_owned()),
            vpc_id: value.vpc_id,
            owner_id: value.owner_id,
            instance_tenancy: value.instance_tenancy.map(|it| it.as_str().to_owned()),
            ipv6_cidr_block_association_set: value
                .ipv6_cidr_block_association_set
                .map(|vec| vec.into_iter().map(|icbas| icbas.into()).collect()),
            cidr_block_association_set: value
                .cidr_block_association_set
                .map(|vec| vec.into_iter().map(|cbas| cbas.into()).collect()),
            is_default: value.is_default,
            // tags: todo!(),
        }
    }
}

#[derive(::std::clone::Clone, ::std::cmp::PartialEq, ::std::fmt::Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VpcIpv6CidrBlockAssociation {
    /// <p>The association ID for the IPv6 CIDR block.</p>
    pub association_id: ::std::option::Option<::std::string::String>,
    /// <p>The IPv6 CIDR block.</p>
    pub ipv6_cidr_block: ::std::option::Option<::std::string::String>,
    /// <p>Information about the state of the CIDR block.</p>
    pub ipv6_cidr_block_state: ::std::option::Option<VpcCidrBlockState>,
    /// <p>The name of the unique set of Availability Zones, Local Zones, or Wavelength Zones from which Amazon Web Services advertises IP addresses, for example, <code>us-east-1-wl1-bos-wlz-1</code>.</p>
    pub network_border_group: ::std::option::Option<::std::string::String>,
    /// <p>The ID of the IPv6 address pool from which the IPv6 CIDR block is allocated.</p>
    pub ipv6_pool: ::std::option::Option<::std::string::String>,
}

impl From<aws_sdk_ec2::types::VpcIpv6CidrBlockAssociation> for VpcIpv6CidrBlockAssociation {
    fn from(value: aws_sdk_ec2::types::VpcIpv6CidrBlockAssociation) -> Self {
        Self {
            association_id: value.association_id,
            ipv6_cidr_block: value.ipv6_cidr_block,
            ipv6_cidr_block_state: value.ipv6_cidr_block_state.map(|icbs| icbs.into()),
            network_border_group: value.network_border_group,
            ipv6_pool: value.ipv6_pool,
        }
    }
}

#[derive(::std::clone::Clone, ::std::cmp::PartialEq, ::std::fmt::Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VpcCidrBlockAssociation {
    /// <p>The association ID for the IPv4 CIDR block.</p>
    pub association_id: ::std::option::Option<::std::string::String>,
    /// <p>The IPv4 CIDR block.</p>
    pub cidr_block: ::std::option::Option<::std::string::String>,
    /// <p>Information about the state of the CIDR block.</p>
    pub cidr_block_state: ::std::option::Option<VpcCidrBlockState>,
}

impl From<aws_sdk_ec2::types::VpcCidrBlockAssociation> for VpcCidrBlockAssociation {
    fn from(value: aws_sdk_ec2::types::VpcCidrBlockAssociation) -> Self {
        Self {
            association_id: value.association_id,
            cidr_block: value.cidr_block,
            cidr_block_state: value.cidr_block_state.map(|cbs| cbs.into()),
        }
    }
}

#[derive(::std::clone::Clone, ::std::cmp::PartialEq, ::std::fmt::Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VpcCidrBlockState {
    /// <p>The state of the CIDR block.</p>
    pub state: ::std::option::Option<String>,
    /// <p>A message about the status of the CIDR block, if applicable.</p>
    pub status_message: ::std::option::Option<::std::string::String>,
}

impl From<aws_sdk_ec2::types::VpcCidrBlockState> for VpcCidrBlockState {
    fn from(value: aws_sdk_ec2::types::VpcCidrBlockState) -> Self {
        Self {
            state: value.state.map(|s| s.as_str().to_owned()),
            status_message: value.status_message,
        }
    }
}

#[derive(::std::clone::Clone, ::std::cmp::PartialEq, ::std::fmt::Debug, Serialize)]
pub struct DescribeFlowLogs {
    pub inner: Vec<FlowLog>,
}

impl ToHecEvents for &DescribeFlowLogs {
    type Item = FlowLog;

    fn source(&self) -> &str {
        "ec2_DescribeFlowLogs"
    }

    fn sourcetype(&self) -> &str {
        "ssphp:aws:json"
    }

    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        Box::new(self.inner.iter())
    }
}

#[derive(::std::clone::Clone, ::std::cmp::PartialEq, ::std::fmt::Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FlowLog {
    /// <p>The date and time the flow log was created.</p>
    #[serde(with = "date_time_def")]
    pub creation_time: ::std::option::Option<::aws_smithy_types::DateTime>,
    /// <p>Information about the error that occurred. <code>Rate limited</code> indicates that CloudWatch Logs throttling has been applied for one or more network interfaces, or that you've reached the limit on the number of log groups that you can create. <code>Access error</code> indicates that the IAM role associated with the flow log does not have sufficient permissions to publish to CloudWatch Logs. <code>Unknown error</code> indicates an internal error.</p>
    pub deliver_logs_error_message: ::std::option::Option<::std::string::String>,
    /// <p>The ARN of the IAM role allows the service to publish logs to CloudWatch Logs.</p>
    pub deliver_logs_permission_arn: ::std::option::Option<::std::string::String>,
    /// <p>The ARN of the IAM role that allows the service to publish flow logs across accounts.</p>
    pub deliver_cross_account_role: ::std::option::Option<::std::string::String>,
    /// <p>The status of the logs delivery (<code>SUCCESS</code> | <code>FAILED</code>).</p>
    pub deliver_logs_status: ::std::option::Option<::std::string::String>,
    /// <p>The ID of the flow log.</p>
    pub flow_log_id: ::std::option::Option<::std::string::String>,
    /// <p>The status of the flow log (<code>ACTIVE</code>).</p>
    pub flow_log_status: ::std::option::Option<::std::string::String>,
    /// <p>The name of the flow log group.</p>
    pub log_group_name: ::std::option::Option<::std::string::String>,
    /// <p>The ID of the resource being monitored.</p>
    pub resource_id: ::std::option::Option<::std::string::String>,
    /// <p>The type of traffic captured for the flow log.</p>
    pub traffic_type: ::std::option::Option<String>,
    /// <p>The type of destination for the flow log data.</p>
    pub log_destination_type: ::std::option::Option<String>,
    /// <p>The Amazon Resource Name (ARN) of the destination for the flow log data.</p>
    pub log_destination: ::std::option::Option<::std::string::String>,
    /// <p>The format of the flow log record.</p>
    pub log_format: ::std::option::Option<::std::string::String>,
    /// <p>The tags for the flow log.</p>
    // pub tags: ::std::option::Option<::std::vec::Vec<crate::types::Tag>>,
    /// <p>The maximum interval of time, in seconds, during which a flow of packets is captured and aggregated into a flow log record.</p>
    /// <p>When a network interface is attached to a <a href="https://docs.aws.amazon.com/AWSEC2/latest/UserGuide/instance-types.html#ec2-nitro-instances">Nitro-based instance</a>, the aggregation interval is always 60 seconds (1 minute) or less, regardless of the specified value.</p>
    /// <p>Valid Values: <code>60</code> | <code>600</code></p>
    pub max_aggregation_interval: ::std::option::Option<i32>,
    /// <p>The destination options.</p>
    pub destination_options: ::std::option::Option<DestinationOptionsResponse>,
}

impl From<aws_sdk_ec2::types::FlowLog> for FlowLog {
    fn from(value: aws_sdk_ec2::types::FlowLog) -> Self {
        Self {
            creation_time: value.creation_time,
            deliver_logs_error_message: value.deliver_logs_error_message,
            deliver_logs_permission_arn: value.deliver_logs_permission_arn,
            deliver_cross_account_role: value.deliver_cross_account_role,
            deliver_logs_status: value.deliver_logs_status,
            flow_log_id: value.flow_log_id,
            flow_log_status: value.flow_log_status,
            log_group_name: value.log_group_name,
            resource_id: value.resource_id,
            traffic_type: value.traffic_type.map(|tt| tt.as_str().to_owned()),
            log_destination_type: value
                .log_destination_type
                .map(|ldt| ldt.as_str().to_owned()),
            log_destination: value.log_destination,
            log_format: value.log_format,
            // tags: value.tags,
            max_aggregation_interval: value.max_aggregation_interval,
            destination_options: value.destination_options.map(|d_o| d_o.into()),
        }
    }
}

#[derive(::std::clone::Clone, ::std::cmp::PartialEq, ::std::fmt::Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DestinationOptionsResponse {
    /// <p>The format for the flow log.</p>
    pub file_format: ::std::option::Option<String>,
    /// <p>Indicates whether to use Hive-compatible prefixes for flow logs stored in Amazon S3.</p>
    pub hive_compatible_partitions: ::std::option::Option<bool>,
    /// <p>Indicates whether to partition the flow log per hour.</p>
    pub per_hour_partition: ::std::option::Option<bool>,
}

impl From<aws_sdk_ec2::types::DestinationOptionsResponse> for DestinationOptionsResponse {
    fn from(value: aws_sdk_ec2::types::DestinationOptionsResponse) -> Self {
        Self {
            file_format: value.file_format.map(|ff| ff.as_str().to_owned()),
            hive_compatible_partitions: value.hive_compatible_partitions,
            per_hour_partition: value.per_hour_partition,
        }
    }
}
