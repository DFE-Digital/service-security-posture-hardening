use data_ingester_splunk::splunk::ToHecEvents;
use serde::Serialize;

#[derive(Default, Debug, Clone, Serialize)]
pub(crate) struct HostedZones {
    pub(crate) inner: Vec<HostedZone>,
}

impl ToHecEvents for &HostedZones {
    type Item = HostedZone;

    fn source(&self) -> &str {
        "iam_ListHostedZones"
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
pub struct HostedZone {
    /// <p>The ID that Amazon Route 53 assigned to the hosted zone when you created it.</p>
    pub id: ::std::string::String,
    /// <p>The name of the domain. For public hosted zones, this is the name that you have registered with your DNS registrar.</p>
    /// <p>For information about how to specify characters other than <code>a-z</code>, <code>0-9</code>, and <code>-</code> (hyphen) and how to specify internationalized domain names, see <a href="https://docs.aws.amazon.com/Route53/latest/APIReference/API_CreateHostedZone.html">CreateHostedZone</a>.</p>
    pub name: ::std::string::String,
    /// <p>The value that you specified for <code>CallerReference</code> when you created the hosted zone.</p>
    pub caller_reference: ::std::string::String,
    /// <p>A complex type that includes the <code>Comment</code> and <code>PrivateZone</code> elements. If you omitted the <code>HostedZoneConfig</code> and <code>Comment</code> elements from the request, the <code>Config</code> and <code>Comment</code> elements don't appear in the response.</p>
    pub config: ::std::option::Option<HostedZoneConfig>,
    /// <p>The number of resource record sets in the hosted zone.</p>
    pub resource_record_set_count: ::std::option::Option<i64>,
    /// <p>If the hosted zone was created by another service, the service that created the hosted zone. When a hosted zone is created by another service, you can't edit or delete it using Route 53.</p>
    pub linked_service: ::std::option::Option<LinkedService>,
    pub(crate) resource_record_sets: Option<Vec<ResourceRecordSet>>,
}

impl From<aws_sdk_route53::types::HostedZone> for HostedZone {
    fn from(value: aws_sdk_route53::types::HostedZone) -> Self {
        Self {
            id: value.id,
            name: value.name,
            caller_reference: value.caller_reference,
            config: value.config.map(|config| config.into()),
            resource_record_set_count: value.resource_record_set_count,
            linked_service: value.linked_service.map(|ls| ls.into()),
            resource_record_sets: None,
        }
    }
}

#[derive(::std::clone::Clone, ::std::cmp::PartialEq, ::std::fmt::Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HostedZoneConfig {
    /// <p>Any comments that you want to include about the hosted zone.</p>
    pub comment: ::std::option::Option<::std::string::String>,
    /// <p>A value that indicates whether this is a private hosted zone.</p>
    pub private_zone: bool,
}

impl From<aws_sdk_route53::types::HostedZoneConfig> for HostedZoneConfig {
    fn from(value: aws_sdk_route53::types::HostedZoneConfig) -> Self {
        Self {
            comment: value.comment,
            private_zone: value.private_zone,
        }
    }
}

#[derive(::std::clone::Clone, ::std::cmp::PartialEq, ::std::fmt::Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkedService {
    /// <p>If the health check or hosted zone was created by another service, the service that created the resource. When a resource is created by another service, you can't edit or delete it using Amazon Route 53.</p>
    pub service_principal: ::std::option::Option<::std::string::String>,
    /// <p>If the health check or hosted zone was created by another service, an optional description that can be provided by the other service. When a resource is created by another service, you can't edit or delete it using Amazon Route 53.</p>
    pub description: ::std::option::Option<::std::string::String>,
}

impl From<aws_sdk_route53::types::LinkedService> for LinkedService {
    fn from(value: aws_sdk_route53::types::LinkedService) -> Self {
        Self {
            service_principal: value.service_principal,
            description: value.description,
        }
    }
}

#[derive(::std::clone::Clone, ::std::cmp::PartialEq, ::std::fmt::Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceRecordSet {
    pub name: ::std::string::String,
    pub r#type: String,
    pub set_identifier: ::std::option::Option<::std::string::String>,
    pub weight: ::std::option::Option<i64>,
    pub region: ::std::option::Option<String>,
    pub geo_location: ::std::option::Option<GeoLocation>,
    pub failover: ::std::option::Option<String>,
    pub multi_value_answer: ::std::option::Option<bool>,
    pub ttl: ::std::option::Option<i64>,
    pub resource_records: ::std::option::Option<::std::vec::Vec<ResourceRecord>>,
    pub alias_target: ::std::option::Option<AliasTarget>,
    pub health_check_id: ::std::option::Option<::std::string::String>,
    pub traffic_policy_instance_id: ::std::option::Option<::std::string::String>,
    pub cidr_routing_config: ::std::option::Option<CidrRoutingConfig>,
    pub geo_proximity_location: ::std::option::Option<GeoProximityLocation>,
}

impl From<aws_sdk_route53::types::ResourceRecordSet> for ResourceRecordSet {
    fn from(value: aws_sdk_route53::types::ResourceRecordSet) -> Self {
        Self {
            name: value.name,
            r#type: value.r#type.as_str().to_string(),
            set_identifier: value.set_identifier,
            weight: value.weight,
            region: value.region.map(|r| r.as_str().to_string()),
            geo_location: value.geo_location.map(|gl| gl.into()),
            failover: value.failover.map(|f| f.as_str().to_string()),
            multi_value_answer: value.multi_value_answer,
            ttl: value.ttl,
            resource_records: value
                .resource_records
                .map(|vec| vec.into_iter().map(|rr| rr.into()).collect()),
            alias_target: value.alias_target.map(|at| at.into()),
            health_check_id: value.health_check_id,
            traffic_policy_instance_id: value.traffic_policy_instance_id,
            cidr_routing_config: value.cidr_routing_config.map(|crc| crc.into()),
            geo_proximity_location: value.geo_proximity_location.map(|gpl| gpl.into()),
        }
    }
}

#[derive(::std::clone::Clone, ::std::cmp::PartialEq, ::std::fmt::Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GeoLocation {
    pub continent_code: ::std::option::Option<::std::string::String>,
    pub country_code: ::std::option::Option<::std::string::String>,
    pub subdivision_code: ::std::option::Option<::std::string::String>,
}

impl From<aws_sdk_route53::types::GeoLocation> for GeoLocation {
    fn from(value: aws_sdk_route53::types::GeoLocation) -> Self {
        Self {
            continent_code: value.continent_code,
            country_code: value.country_code,
            subdivision_code: value.subdivision_code,
        }
    }
}

#[derive(::std::clone::Clone, ::std::cmp::PartialEq, ::std::fmt::Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceRecord {
    pub value: ::std::string::String,
}

impl From<aws_sdk_route53::types::ResourceRecord> for ResourceRecord {
    fn from(value: aws_sdk_route53::types::ResourceRecord) -> Self {
        Self { value: value.value }
    }
}

#[derive(::std::clone::Clone, ::std::cmp::PartialEq, ::std::fmt::Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AliasTarget {
    pub hosted_zone_id: ::std::string::String,
    pub dns_name: ::std::string::String,
    pub evaluate_target_health: bool,
}

impl From<aws_sdk_route53::types::AliasTarget> for AliasTarget {
    fn from(value: aws_sdk_route53::types::AliasTarget) -> Self {
        Self {
            hosted_zone_id: value.hosted_zone_id,
            dns_name: value.dns_name,
            evaluate_target_health: value.evaluate_target_health,
        }
    }
}

#[derive(::std::clone::Clone, ::std::cmp::PartialEq, ::std::fmt::Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CidrRoutingConfig {
    pub collection_id: ::std::string::String,
    pub location_name: ::std::string::String,
}

impl From<aws_sdk_route53::types::CidrRoutingConfig> for CidrRoutingConfig {
    fn from(value: aws_sdk_route53::types::CidrRoutingConfig) -> Self {
        Self {
            collection_id: value.collection_id,
            location_name: value.location_name,
        }
    }
}

#[derive(::std::clone::Clone, ::std::cmp::PartialEq, ::std::fmt::Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GeoProximityLocation {
    pub aws_region: ::std::option::Option<::std::string::String>,
    pub local_zone_group: ::std::option::Option<::std::string::String>,
    pub coordinates: ::std::option::Option<Coordinates>,
    pub bias: ::std::option::Option<i32>,
}

impl From<aws_sdk_route53::types::GeoProximityLocation> for GeoProximityLocation {
    fn from(value: aws_sdk_route53::types::GeoProximityLocation) -> Self {
        Self {
            aws_region: value.aws_region,
            local_zone_group: value.local_zone_group,
            coordinates: value.coordinates.map(|c| c.into()),
            bias: value.bias,
        }
    }
}

#[derive(::std::clone::Clone, ::std::cmp::PartialEq, ::std::fmt::Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Coordinates {
    pub latitude: ::std::string::String,
    pub longitude: ::std::string::String,
}

impl From<aws_sdk_route53::types::Coordinates> for Coordinates {
    fn from(value: aws_sdk_route53::types::Coordinates) -> Self {
        Self {
            latitude: value.latitude,
            longitude: value.longitude,
        }
    }
}
