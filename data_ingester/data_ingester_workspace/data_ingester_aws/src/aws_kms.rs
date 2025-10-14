use serde::Serialize;

use crate::aws_trail::date_time_def;
use data_ingester_splunk::splunk::ToHecEvents;

// #[derive(::std::clone::Clone, ::std::cmp::PartialEq, ::std::fmt::Debug, Serialize)]
// pub struct DescribeKeyOutput {
//     /// <p>Metadata associated with the key.</p>
//     pub key_metadata: ::std::option::Option<KeyMetadata>,
// }

// impl From<aws_sdk_kms::operation::describe_key::DescribeKeyOutput> for DescribeKeyOutput {
//     fn from(value: aws_sdk_kms::operation::describe_key::DescribeKeyOutput) -> Self {
//         Self {
//             key_metadata: value.key_metadata.map(|km| km.into()),
//         }
//     }
// }

#[derive(::std::clone::Clone, ::std::cmp::PartialEq, ::std::fmt::Debug, Serialize)]
pub struct KeyMetadatas {
    pub inner: Vec<KeyMetadata>,
}

impl ToHecEvents for &KeyMetadatas {
    type Item = KeyMetadata;

    fn source(&self) -> &str {
        "kms_DescribeKey"
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

#[derive(::std::clone::Clone, ::std::cmp::PartialEq, ::std::fmt::Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyMetadata {
    /// <p>The twelve-digit account ID of the Amazon Web Services account that owns the KMS key.</p>
    pub aws_account_id: ::std::option::Option<::std::string::String>,
    /// <p>The globally unique identifier for the KMS key.</p>
    pub key_id: ::std::string::String,
    /// <p>The Amazon Resource Name (ARN) of the KMS key. For examples, see <a href="https://docs.aws.amazon.com/general/latest/gr/aws-arns-and-namespaces.html#arn-syntax-kms">Key Management Service (KMS)</a> in the Example ARNs section of the <i>Amazon Web Services General Reference</i>.</p>
    pub arn: ::std::option::Option<::std::string::String>,
    /// <p>The date and time when the KMS key was created.</p>
    #[serde(with = "date_time_def")]
    pub creation_date: ::std::option::Option<::aws_smithy_types::DateTime>,
    /// <p>Specifies whether the KMS key is enabled. When <code>KeyState</code> is <code>Enabled</code> this value is true, otherwise it is false.</p>
    pub enabled: bool,
    /// <p>The description of the KMS key.</p>
    pub description: ::std::option::Option<::std::string::String>,
    /// <p>The <a href="https://docs.aws.amazon.com/kms/latest/developerguide/concepts.html#cryptographic-operations">cryptographic operations</a> for which you can use the KMS key.</p>
    pub key_usage: ::std::option::Option<String>,
    /// <p>The current status of the KMS key.</p>
    /// <p>For more information about how key state affects the use of a KMS key, see <a href="https://docs.aws.amazon.com/kms/latest/developerguide/key-state.html">Key states of KMS keys</a> in the <i>Key Management Service Developer Guide</i>.</p>
    pub key_state: ::std::option::Option<String>,
    /// <p>The date and time after which KMS deletes this KMS key. This value is present only when the KMS key is scheduled for deletion, that is, when its <code>KeyState</code> is <code>PendingDeletion</code>.</p>
    /// <p>When the primary key in a multi-Region key is scheduled for deletion but still has replica keys, its key state is <code>PendingReplicaDeletion</code> and the length of its waiting period is displayed in the <code>PendingDeletionWindowInDays</code> field.</p>
    #[serde(with = "date_time_def")]
    pub deletion_date: ::std::option::Option<::aws_smithy_types::DateTime>,
    /// <p>The time at which the imported key material expires. When the key material expires, KMS deletes the key material and the KMS key becomes unusable. This value is present only for KMS keys whose <code>Origin</code> is <code>EXTERNAL</code> and whose <code>ExpirationModel</code> is <code>KEY_MATERIAL_EXPIRES</code>, otherwise this value is omitted.</p>
    #[serde(with = "date_time_def")]
    pub valid_to: ::std::option::Option<::aws_smithy_types::DateTime>,
    /// <p>The source of the key material for the KMS key. When this value is <code>AWS_KMS</code>, KMS created the key material. When this value is <code>EXTERNAL</code>, the key material was imported or the KMS key doesn't have any key material. When this value is <code>AWS_CLOUDHSM</code>, the key material was created in the CloudHSM cluster associated with a custom key store.</p>
    pub origin: ::std::option::Option<String>,
    /// <p>A unique identifier for the <a href="https://docs.aws.amazon.com/kms/latest/developerguide/custom-key-store-overview.html">custom key store</a> that contains the KMS key. This field is present only when the KMS key is created in a custom key store.</p>
    pub custom_key_store_id: ::std::option::Option<::std::string::String>,
    /// <p>The cluster ID of the CloudHSM cluster that contains the key material for the KMS key. When you create a KMS key in an CloudHSM <a href="https://docs.aws.amazon.com/kms/latest/developerguide/custom-key-store-overview.html">custom key store</a>, KMS creates the key material for the KMS key in the associated CloudHSM cluster. This field is present only when the KMS key is created in an CloudHSM key store.</p>
    pub cloud_hsm_cluster_id: ::std::option::Option<::std::string::String>,
    /// <p>Specifies whether the KMS key's key material expires. This value is present only when <code>Origin</code> is <code>EXTERNAL</code>, otherwise this value is omitted.</p>
    pub expiration_model: ::std::option::Option<String>,
    /// <p>The manager of the KMS key. KMS keys in your Amazon Web Services account are either customer managed or Amazon Web Services managed. For more information about the difference, see <a href="https://docs.aws.amazon.com/kms/latest/developerguide/concepts.html#kms_keys">KMS keys</a> in the <i>Key Management Service Developer Guide</i>.</p>
    pub key_manager: ::std::option::Option<String>,
    /// <p>Instead, use the <code>KeySpec</code> field.</p>
    /// <p>The <code>KeySpec</code> and <code>CustomerMasterKeySpec</code> fields have the same value. We recommend that you use the <code>KeySpec</code> field in your code. However, to avoid breaking changes, KMS supports both fields.</p>
    // #[deprecated(note = "This field has been deprecated. Instead, use the KeySpec field.")]
    // pub customer_master_key_spec: ::std::option::Option<String>,
    /// <p>Describes the type of key material in the KMS key.</p>
    pub key_spec: ::std::option::Option<String>,
    /// <p>The encryption algorithms that the KMS key supports. You cannot use the KMS key with other encryption algorithms within KMS.</p>
    /// <p>This value is present only when the <code>KeyUsage</code> of the KMS key is <code>ENCRYPT_DECRYPT</code>.</p>
    pub encryption_algorithms: ::std::option::Option<::std::vec::Vec<String>>,
    /// <p>The signing algorithms that the KMS key supports. You cannot use the KMS key with other signing algorithms within KMS.</p>
    /// <p>This field appears only when the <code>KeyUsage</code> of the KMS key is <code>SIGN_VERIFY</code>.</p>
    pub signing_algorithms: ::std::option::Option<::std::vec::Vec<String>>,
    /// <p>Indicates whether the KMS key is a multi-Region (<code>True</code>) or regional (<code>False</code>) key. This value is <code>True</code> for multi-Region primary and replica keys and <code>False</code> for regional KMS keys.</p>
    /// <p>For more information about multi-Region keys, see <a href="https://docs.aws.amazon.com/kms/latest/developerguide/multi-region-keys-overview.html">Multi-Region keys in KMS</a> in the <i>Key Management Service Developer Guide</i>.</p>
    pub multi_region: ::std::option::Option<bool>,
    /// <p>Lists the primary and replica keys in same multi-Region key. This field is present only when the value of the <code>MultiRegion</code> field is <code>True</code>.</p>
    /// <p>For more information about any listed KMS key, use the <code>DescribeKey</code> operation.</p>
    /// <ul>
    /// <li>
    /// <p><code>MultiRegionKeyType</code> indicates whether the KMS key is a <code>PRIMARY</code> or <code>REPLICA</code> key.</p></li>
    /// <li>
    /// <p><code>PrimaryKey</code> displays the key ARN and Region of the primary key. This field displays the current KMS key if it is the primary key.</p></li>
    /// <li>
    /// <p><code>ReplicaKeys</code> displays the key ARNs and Regions of all replica keys. This field includes the current KMS key if it is a replica key.</p></li>
    /// </ul>
    pub multi_region_configuration: ::std::option::Option<MultiRegionConfiguration>,
    /// <p>The waiting period before the primary key in a multi-Region key is deleted. This waiting period begins when the last of its replica keys is deleted. This value is present only when the <code>KeyState</code> of the KMS key is <code>PendingReplicaDeletion</code>. That indicates that the KMS key is the primary key in a multi-Region key, it is scheduled for deletion, and it still has existing replica keys.</p>
    /// <p>When a single-Region KMS key or a multi-Region replica key is scheduled for deletion, its deletion date is displayed in the <code>DeletionDate</code> field. However, when the primary key in a multi-Region key is scheduled for deletion, its waiting period doesn't begin until all of its replica keys are deleted. This value displays that waiting period. When the last replica key in the multi-Region key is deleted, the <code>KeyState</code> of the scheduled primary key changes from <code>PendingReplicaDeletion</code> to <code>PendingDeletion</code> and the deletion date appears in the <code>DeletionDate</code> field.</p>
    pub pending_deletion_window_in_days: ::std::option::Option<i32>,
    /// <p>The message authentication code (MAC) algorithm that the HMAC KMS key supports.</p>
    /// <p>This value is present only when the <code>KeyUsage</code> of the KMS key is <code>GENERATE_VERIFY_MAC</code>.</p>
    pub mac_algorithms: ::std::option::Option<::std::vec::Vec<String>>,
    /// <p>Information about the external key that is associated with a KMS key in an external key store.</p>
    /// <p>For more information, see <a href="https://docs.aws.amazon.com/kms/latest/developerguide/keystore-external.html#concept-external-key">External key</a> in the <i>Key Management Service Developer Guide</i>.</p>
    pub xks_key_configuration: ::std::option::Option<XksKeyConfigurationType>,
    pub key_rotation_status: Option<GetKeyRotationStatusOutput>,
}

impl From<aws_sdk_kms::types::KeyMetadata> for KeyMetadata {
    fn from(value: aws_sdk_kms::types::KeyMetadata) -> Self {
        Self {
            aws_account_id: value.aws_account_id,
            key_id: value.key_id,
            arn: value.arn,
            creation_date: value.creation_date,
            enabled: value.enabled,
            description: value.description,
            key_usage: value.key_usage.map(|ku| ku.as_str().to_owned()),
            key_state: value.key_state.map(|ks| ks.as_str().to_owned()),
            deletion_date: value.deletion_date,
            valid_to: value.valid_to,
            origin: value.origin.map(|o| o.as_str().to_owned()),
            custom_key_store_id: value.custom_key_store_id,
            cloud_hsm_cluster_id: value.cloud_hsm_cluster_id,
            expiration_model: value.expiration_model.map(|em| em.as_str().to_owned()),
            key_manager: value.key_manager.map(|km| km.as_str().to_owned()),
            // customer_master_key_spec: value
            //     .customer_master_key_spec
            //     .map(|cmks| cmks.as_str().to_owned()),
            key_spec: value.key_spec.map(|ks| ks.as_str().to_owned()),
            encryption_algorithms: value
                .encryption_algorithms
                .map(|vec| vec.into_iter().map(|ea| ea.as_str().to_owned()).collect()),
            signing_algorithms: value
                .signing_algorithms
                .map(|vec| vec.into_iter().map(|sa| sa.as_str().to_owned()).collect()),
            multi_region: value.multi_region,
            multi_region_configuration: value.multi_region_configuration.map(|mrc| mrc.into()),
            pending_deletion_window_in_days: value.pending_deletion_window_in_days,
            mac_algorithms: value
                .mac_algorithms
                .map(|vec| vec.into_iter().map(|ma| ma.as_str().to_owned()).collect()),
            xks_key_configuration: value.xks_key_configuration.map(|xkc| xkc.into()),
            key_rotation_status: None,
        }
    }
}

#[derive(::std::clone::Clone, ::std::cmp::PartialEq, ::std::fmt::Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetKeyRotationStatusOutput {
    /// <p>A Boolean value that specifies whether key rotation is enabled.</p>
    pub key_rotation_enabled: bool,
}

impl From<aws_sdk_kms::operation::get_key_rotation_status::GetKeyRotationStatusOutput>
    for GetKeyRotationStatusOutput
{
    fn from(
        value: aws_sdk_kms::operation::get_key_rotation_status::GetKeyRotationStatusOutput,
    ) -> Self {
        Self {
            key_rotation_enabled: value.key_rotation_enabled,
        }
    }
}

#[derive(::std::clone::Clone, ::std::cmp::PartialEq, ::std::fmt::Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MultiRegionConfiguration {
    /// <p>Indicates whether the KMS key is a <code>PRIMARY</code> or <code>REPLICA</code> key.</p>
    pub multi_region_key_type: ::std::option::Option<String>,
    /// <p>Displays the key ARN and Region of the primary key. This field includes the current KMS key if it is the primary key.</p>
    pub primary_key: ::std::option::Option<MultiRegionKey>,
    /// <p>displays the key ARNs and Regions of all replica keys. This field includes the current KMS key if it is a replica key.</p>
    pub replica_keys: ::std::option::Option<::std::vec::Vec<MultiRegionKey>>,
}

impl From<aws_sdk_kms::types::MultiRegionConfiguration> for MultiRegionConfiguration {
    fn from(value: aws_sdk_kms::types::MultiRegionConfiguration) -> Self {
        Self {
            multi_region_key_type: value
                .multi_region_key_type
                .map(|mrkt| mrkt.as_str().to_owned()),
            primary_key: value.primary_key.map(|mrk| mrk.into()),
            replica_keys: value
                .replica_keys
                .map(|vec| vec.into_iter().map(|mrk| mrk.into()).collect()),
        }
    }
}

#[derive(::std::clone::Clone, ::std::cmp::PartialEq, ::std::fmt::Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MultiRegionKey {
    /// <p>Displays the key ARN of a primary or replica key of a multi-Region key.</p>
    pub arn: ::std::option::Option<::std::string::String>,
    /// <p>Displays the Amazon Web Services Region of a primary or replica key in a multi-Region key.</p>
    pub region: ::std::option::Option<::std::string::String>,
}

impl From<aws_sdk_kms::types::MultiRegionKey> for MultiRegionKey {
    fn from(value: aws_sdk_kms::types::MultiRegionKey) -> Self {
        Self {
            arn: value.arn,
            region: value.region,
        }
    }
}

#[derive(::std::clone::Clone, ::std::cmp::PartialEq, ::std::fmt::Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct XksKeyConfigurationType {
    /// <p>The ID of the external key in its external key manager. This is the ID that the external key store proxy uses to identify the external key.</p>
    pub id: ::std::option::Option<::std::string::String>,
}

impl From<aws_sdk_kms::types::XksKeyConfigurationType> for XksKeyConfigurationType {
    fn from(value: aws_sdk_kms::types::XksKeyConfigurationType) -> Self {
        Self { id: value.id }
    }
}
