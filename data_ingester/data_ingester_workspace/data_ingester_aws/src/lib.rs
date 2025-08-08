pub mod aws;
mod aws_alternate_contact_information;
mod aws_config;
mod aws_ec2;
mod aws_entities_for_policy;
mod aws_iam;
mod aws_kms;
mod aws_organizations;
mod aws_policy;
mod aws_route53;
mod aws_s3;
mod aws_s3control;
mod aws_securityhub;
mod aws_trail;

pub static SSPHP_RUN_KEY: &str = "aws";
