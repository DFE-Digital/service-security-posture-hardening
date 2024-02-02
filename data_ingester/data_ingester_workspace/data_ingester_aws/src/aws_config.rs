use serde::Serialize;

use crate::aws_trail::date_time_def;
use data_ingester_splunk::splunk::ToHecEvents;

#[derive(::std::clone::Clone, ::std::cmp::PartialEq, ::std::fmt::Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct DescribeConfigurationRecordersOutput {
    /// <p>A list that contains the descriptions of the specified configuration recorders.</p>
    pub configuration_recorders: ::std::option::Option<::std::vec::Vec<ConfigurationRecorder>>,
}

impl ToHecEvents for &DescribeConfigurationRecordersOutput {
    type Item = ConfigurationRecorder;

    fn source(&self) -> &str {
        "config_DescribeConfigurationRecordersOutput"
    }

    fn sourcetype(&self) -> &str {
        "ssphp:aws:json"
    }

    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        match self.configuration_recorders {
            Some(ref vec) => Box::new(vec.iter()),
            None => Box::new(std::iter::empty()),
        }
    }
}

impl From<aws_sdk_config::operation::describe_configuration_recorders::DescribeConfigurationRecordersOutput> for DescribeConfigurationRecordersOutput {
    fn from(value: aws_sdk_config::operation::describe_configuration_recorders::DescribeConfigurationRecordersOutput) -> Self {
        Self {
            configuration_recorders: value.configuration_recorders.map(|vec| vec.into_iter().map(|cr| cr.into()).collect()),
        }
    }
}

#[derive(::std::clone::Clone, ::std::cmp::PartialEq, ::std::fmt::Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct ConfigurationRecorder {
    /// <p>The name of the configuration recorder. Config automatically assigns the name of "default" when creating the configuration recorder.</p><note>
    /// <p>You cannot change the name of the configuration recorder after it has been created. To change the configuration recorder name, you must delete it and create a new configuration recorder with a new name.</p>
    /// </note>
    pub name: ::std::option::Option<::std::string::String>,
    /// <p>Amazon Resource Name (ARN) of the IAM role assumed by Config and used by the configuration recorder.</p><note>
    /// <p>While the API model does not require this field, the server will reject a request without a defined <code>roleARN</code> for the configuration recorder.</p>
    /// </note> <note>
    /// <p><b>Pre-existing Config role</b></p>
    /// <p>If you have used an Amazon Web Services service that uses Config, such as Security Hub or Control Tower, and an Config role has already been created, make sure that the IAM role that you use when setting up Config keeps the same minimum permissions as the already created Config role. You must do this so that the other Amazon Web Services service continues to run as expected.</p>
    /// <p>For example, if Control Tower has an IAM role that allows Config to read Amazon Simple Storage Service (Amazon S3) objects, make sure that the same permissions are granted within the IAM role you use when setting up Config. Otherwise, it may interfere with how Control Tower operates. For more information about IAM roles for Config, see <a href="https://docs.aws.amazon.com/config/latest/developerguide/security-iam.html"> <b>Identity and Access Management for Config</b> </a> in the <i>Config Developer Guide</i>.</p>
    /// </note>
    pub role_arn: ::std::option::Option<::std::string::String>,
    /// <p>Specifies which resource types Config records for configuration changes.</p><note>
    /// <p><b> High Number of Config Evaluations</b></p>
    /// <p>You may notice increased activity in your account during your initial month recording with Config when compared to subsequent months. During the initial bootstrapping process, Config runs evaluations on all the resources in your account that you have selected for Config to record.</p>
    /// <p>If you are running ephemeral workloads, you may see increased activity from Config as it records configuration changes associated with creating and deleting these temporary resources. An <i>ephemeral workload</i> is a temporary use of computing resources that are loaded and run when needed. Examples include Amazon Elastic Compute Cloud (Amazon EC2) Spot Instances, Amazon EMR jobs, and Auto Scaling. If you want to avoid the increased activity from running ephemeral workloads, you can run these types of workloads in a separate account with Config turned off to avoid increased configuration recording and rule evaluations.</p>
    /// </note>
    pub recording_group: ::std::option::Option<RecordingGroup>,
    /// <p>Specifies the default recording frequency that Config uses to record configuration changes. Config supports <i>Continuous recording</i> and <i>Daily recording</i>.</p>
    /// <ul>
    /// <li>
    /// <p>Continuous recording allows you to record configuration changes continuously whenever a change occurs.</p></li>
    /// <li>
    /// <p>Daily recording allows you to receive a configuration item (CI) representing the most recent state of your resources over the last 24-hour period, only if it’s different from the previous CI recorded.</p></li>
    /// </ul><note>
    /// <p>Firewall Manager depends on continuous recording to monitor your resources. If you are using Firewall Manager, it is recommended that you set the recording frequency to Continuous.</p>
    /// </note>
    /// <p>You can also override the recording frequency for specific resource types.</p>
    pub recording_mode: ::std::option::Option<RecordingMode>,
    pub status: Option<Vec<ConfigurationRecorderStatus>>,
}

impl From<aws_sdk_config::types::ConfigurationRecorder> for ConfigurationRecorder {
    fn from(value: aws_sdk_config::types::ConfigurationRecorder) -> Self {
        Self {
            name: value.name,
            role_arn: value.role_arn,
            recording_group: value.recording_group.map(|rg| rg.into()),
            recording_mode: value.recording_mode.map(|rm| rm.into()),
            status: None,
        }
    }
}

#[derive(::std::clone::Clone, ::std::cmp::PartialEq, ::std::fmt::Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct RecordingGroup {
    pub all_supported: bool,
    pub include_global_resource_types: bool,
    pub resource_types: ::std::option::Option<::std::vec::Vec<String>>,
    pub exclusion_by_resource_types: ::std::option::Option<ExclusionByResourceTypes>,
    pub recording_strategy: ::std::option::Option<RecordingStrategy>,
}

impl From<aws_sdk_config::types::RecordingGroup> for RecordingGroup {
    fn from(value: aws_sdk_config::types::RecordingGroup) -> Self {
        Self {
            all_supported: value.all_supported,
            include_global_resource_types: value.include_global_resource_types,
            resource_types: value
                .resource_types
                .map(|vec| vec.into_iter().map(|rt| rt.as_str().to_owned()).collect()),
            exclusion_by_resource_types: value.exclusion_by_resource_types.map(|ebrt| ebrt.into()),
            recording_strategy: value.recording_strategy.map(|rs| rs.into()),
        }
    }
}

#[derive(::std::clone::Clone, ::std::cmp::PartialEq, ::std::fmt::Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct ExclusionByResourceTypes {
    /// <p>A comma-separated list of resource types to exclude from recording by the configuration recorder.</p>
    pub resource_types: ::std::option::Option<::std::vec::Vec<String>>,
}

impl From<aws_sdk_config::types::ExclusionByResourceTypes> for ExclusionByResourceTypes {
    fn from(value: aws_sdk_config::types::ExclusionByResourceTypes) -> Self {
        Self {
            resource_types: value
                .resource_types
                .map(|vec| vec.into_iter().map(|rt| rt.as_str().to_owned()).collect()),
        }
    }
}

#[derive(::std::clone::Clone, ::std::cmp::PartialEq, ::std::fmt::Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct RecordingMode {
    /// <p>The default recording frequency that Config uses to record configuration changes.</p><important>
    /// <p>Daily recording is not supported for the following resource types:</p>
    /// <ul>
    /// <li>
    /// <p><code>AWS::Config::ResourceCompliance</code></p></li>
    /// <li>
    /// <p><code>AWS::Config::ConformancePackCompliance</code></p></li>
    /// <li>
    /// <p><code>AWS::Config::ConfigurationRecorder</code></p></li>
    /// </ul>
    /// <p>For the <b>allSupported</b> (<code>ALL_SUPPORTED_RESOURCE_TYPES</code>) recording strategy, these resource types will be set to Continuous recording.</p>
    /// </important>
    pub recording_frequency: String,
    /// <p>An array of <code>recordingModeOverride</code> objects for you to specify your overrides for the recording mode. The <code>recordingModeOverride</code> object in the <code>recordingModeOverrides</code> array consists of three fields: a <code>description</code>, the new <code>recordingFrequency</code>, and an array of <code>resourceTypes</code> to override.</p>
    pub recording_mode_overrides: ::std::option::Option<::std::vec::Vec<RecordingModeOverride>>,
}

impl From<aws_sdk_config::types::RecordingMode> for RecordingMode {
    fn from(value: aws_sdk_config::types::RecordingMode) -> Self {
        Self {
            recording_frequency: value.recording_frequency.as_str().to_owned(),
            recording_mode_overrides: value
                .recording_mode_overrides
                .map(|vec| vec.into_iter().map(|rmo| rmo.into()).collect()),
        }
    }
}

#[derive(::std::clone::Clone, ::std::cmp::PartialEq, ::std::fmt::Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct RecordingModeOverride {
    /// <p>A description that you provide for the override.</p>
    pub description: ::std::option::Option<::std::string::String>,
    /// <p>A comma-separated list that specifies which resource types Config includes in the override.</p><important>
    /// <p>Daily recording is not supported for the following resource types:</p>
    /// <ul>
    /// <li>
    /// <p><code>AWS::Config::ResourceCompliance</code></p></li>
    /// <li>
    /// <p><code>AWS::Config::ConformancePackCompliance</code></p></li>
    /// <li>
    /// <p><code>AWS::Config::ConfigurationRecorder</code></p></li>
    /// </ul>
    /// </important>
    pub resource_types: ::std::vec::Vec<String>,
    /// <p>The recording frequency that will be applied to all the resource types specified in the override.</p>
    /// <ul>
    /// <li>
    /// <p>Continuous recording allows you to record configuration changes continuously whenever a change occurs.</p></li>
    /// <li>
    /// <p>Daily recording allows you to receive a configuration item (CI) representing the most recent state of your resources over the last 24-hour period, only if it’s different from the previous CI recorded.</p></li>
    /// </ul><note>
    /// <p>Firewall Manager depends on continuous recording to monitor your resources. If you are using Firewall Manager, it is recommended that you set the recording frequency to Continuous.</p>
    /// </note>
    pub recording_frequency: String,
}

impl From<aws_sdk_config::types::RecordingModeOverride> for RecordingModeOverride {
    fn from(value: aws_sdk_config::types::RecordingModeOverride) -> Self {
        Self {
            description: value.description,
            resource_types: value
                .resource_types
                .into_iter()
                .map(|rt| rt.as_str().to_owned())
                .collect(),
            recording_frequency: value.recording_frequency.as_str().to_owned(),
        }
    }
}

#[derive(::std::clone::Clone, ::std::cmp::PartialEq, ::std::fmt::Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct RecordingStrategy {
    /// <p>The recording strategy for the configuration recorder.</p>
    /// <ul>
    /// <li>
    /// <p>If you set this option to <code>ALL_SUPPORTED_RESOURCE_TYPES</code>, Config records configuration changes for all supported resource types, excluding the global IAM resource types. You also must set the <code>allSupported</code> field of <a href="https://docs.aws.amazon.com/config/latest/APIReference/API_RecordingGroup.html">RecordingGroup</a> to <code>true</code>. When Config adds support for a new resource type, Config automatically starts recording resources of that type. For a list of supported resource types, see <a href="https://docs.aws.amazon.com/config/latest/developerguide/resource-config-reference.html#supported-resources">Supported Resource Types</a> in the <i>Config developer guide</i>.</p></li>
    /// <li>
    /// <p>If you set this option to <code>INCLUSION_BY_RESOURCE_TYPES</code>, Config records configuration changes for only the resource types that you specify in the <code>resourceTypes</code> field of <a href="https://docs.aws.amazon.com/config/latest/APIReference/API_RecordingGroup.html">RecordingGroup</a>.</p></li>
    /// <li>
    /// <p>If you set this option to <code>EXCLUSION_BY_RESOURCE_TYPES</code>, Config records configuration changes for all supported resource types, except the resource types that you specify to exclude from being recorded in the <code>resourceTypes</code> field of <a href="https://docs.aws.amazon.com/config/latest/APIReference/API_ExclusionByResourceTypes.html">ExclusionByResourceTypes</a>.</p></li>
    /// </ul><note>
    /// <p><b>Required and optional fields</b></p>
    /// <p>The <code>recordingStrategy</code> field is optional when you set the <code>allSupported</code> field of <a href="https://docs.aws.amazon.com/config/latest/APIReference/API_RecordingGroup.html">RecordingGroup</a> to <code>true</code>.</p>
    /// <p>The <code>recordingStrategy</code> field is optional when you list resource types in the <code>resourceTypes</code> field of <a href="https://docs.aws.amazon.com/config/latest/APIReference/API_RecordingGroup.html">RecordingGroup</a>.</p>
    /// <p>The <code>recordingStrategy</code> field is required if you list resource types to exclude from recording in the <code>resourceTypes</code> field of <a href="https://docs.aws.amazon.com/config/latest/APIReference/API_ExclusionByResourceTypes.html">ExclusionByResourceTypes</a>.</p>
    /// </note> <note>
    /// <p><b>Overriding fields</b></p>
    /// <p>If you choose <code>EXCLUSION_BY_RESOURCE_TYPES</code> for the recording strategy, the <code>exclusionByResourceTypes</code> field will override other properties in the request.</p>
    /// <p>For example, even if you set <code>includeGlobalResourceTypes</code> to false, global IAM resource types will still be automatically recorded in this option unless those resource types are specifically listed as exclusions in the <code>resourceTypes</code> field of <code>exclusionByResourceTypes</code>.</p>
    /// </note> <note>
    /// <p><b>Global resource types and the exclusion recording strategy</b></p>
    /// <p>By default, if you choose the <code>EXCLUSION_BY_RESOURCE_TYPES</code> recording strategy, when Config adds support for a new resource type in the Region where you set up the configuration recorder, including global resource types, Config starts recording resources of that type automatically.</p>
    /// <p>Unless specifically listed as exclusions, <code>AWS::RDS::GlobalCluster</code> will be recorded automatically in all supported Config Regions were the configuration recorder is enabled.</p>
    /// <p>IAM users, groups, roles, and customer managed policies will be recorded in the Region where you set up the configuration recorder if that is a Region where Config was available before February 2022. You cannot be record the global IAM resouce types in Regions supported by Config after February 2022. This list where you cannot record the global IAM resource types includes the following Regions:</p>
    /// <ul>
    /// <li>
    /// <p>Asia Pacific (Hyderabad)</p></li>
    /// <li>
    /// <p>Asia Pacific (Melbourne)</p></li>
    /// <li>
    /// <p>Europe (Spain)</p></li>
    /// <li>
    /// <p>Europe (Zurich)</p></li>
    /// <li>
    /// <p>Israel (Tel Aviv)</p></li>
    /// <li>
    /// <p>Middle East (UAE)</p></li>
    /// </ul>
    /// </note>
    pub use_only: ::std::option::Option<String>,
}

impl From<aws_sdk_config::types::RecordingStrategy> for RecordingStrategy {
    fn from(value: aws_sdk_config::types::RecordingStrategy) -> Self {
        Self {
            use_only: value.use_only.map(|uo| uo.as_str().to_owned()),
        }
    }
}

#[derive(::std::clone::Clone, ::std::cmp::PartialEq, ::std::fmt::Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct ConfigurationRecorderStatus {
    /// <p>The name of the configuration recorder.</p>
    pub name: ::std::option::Option<::std::string::String>,
    /// <p>The time the recorder was last started.</p>
    #[serde(with = "date_time_def")]
    pub last_start_time: ::std::option::Option<::aws_smithy_types::DateTime>,
    /// <p>The time the recorder was last stopped.</p>
    #[serde(with = "date_time_def")]
    pub last_stop_time: ::std::option::Option<::aws_smithy_types::DateTime>,
    /// <p>Specifies whether or not the recorder is currently recording.</p>
    pub recording: bool,
    /// <p>The status of the latest recording event processed by the recorder.</p>
    pub last_status: ::std::option::Option<String>,
    /// <p>The latest error code from when the recorder last failed.</p>
    pub last_error_code: ::std::option::Option<::std::string::String>,
    /// <p>The latest error message from when the recorder last failed.</p>
    pub last_error_message: ::std::option::Option<::std::string::String>,
    /// <p>The time of the latest change in status of an recording event processed by the recorder.</p>
    #[serde(with = "date_time_def")]
    pub last_status_change_time: ::std::option::Option<::aws_smithy_types::DateTime>,
}

impl From<aws_sdk_config::types::ConfigurationRecorderStatus> for ConfigurationRecorderStatus {
    fn from(value: aws_sdk_config::types::ConfigurationRecorderStatus) -> Self {
        Self {
            name: value.name,
            last_start_time: value.last_start_time,
            last_stop_time: value.last_stop_time,
            recording: value.recording,
            last_status: value.last_status.map(|ls| ls.as_str().to_owned()),
            last_error_code: value.last_error_code,
            last_error_message: value.last_error_message,
            last_status_change_time: value.last_status_change_time,
        }
    }
}
