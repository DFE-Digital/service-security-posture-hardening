# │ Error: creating IAM Virtual MFA Device (sam-ca-mfa-device): InvalidClientTokenId: The security token included in the request is invalid
# │ 	status code: 403, request id: ea10949b-28a0-49f1-9fbc-32cba64d550a
# use --no-session

terraform {
  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = "~> 4.16"
    }
  }

  required_version = ">= 1.2.0"
}

provider "aws" {
  region = "eu-west-2"
}

resource "aws_s3_bucket" "example" {
  bucket = "my-tf-ca-test"
}

resource "aws_s3_bucket_public_access_block" "example_acl" {
  bucket = aws_s3_bucket.example.id

  block_public_acls       = true
  block_public_policy     = true
  ignore_public_acls      = true
  restrict_public_buckets = true
}

# resource "aws_s3_bucket_versioning" "bucket_versioning_CIS-2-1-2" {
#   bucket = aws_s3_bucket.example.id

#   versioning_configuration {
#     status = "Enabled"
#     mfa_delete = "Enabled"
#   }
# }

data "aws_iam_policy_document" "aws_iam_policy_doc_s3_example" {
  statement {
    sid    = "AWSDenyS3HTTPRequests_CIS2-1-1"
    effect = "Deny"

    principals {
      type        = "*"
      identifiers = ["*"]
    }

    actions   = ["s3:*"]
    resources = [aws_s3_bucket.example.arn]
    condition {
      test     = "Bool"
      variable = "aws:SecureTransport"
      values   = ["false"]
    }
  }
}

resource "aws_s3_bucket_policy" "s3_bucket_policy_example" {
  bucket = aws_s3_bucket.example.id
  policy = data.aws_iam_policy_document.aws_iam_policy_doc_s3_example.json
}

resource "aws_account_alternate_contact" "security" {

  alternate_contact_type = "SECURITY"

  name          = "Sam Pritchard"
  title         = "Lord"
  email_address = "sam@prigital.co.uk"
  phone_number  = "+447733333555"
}

resource "aws_iam_account_password_policy" "strict" {
  minimum_password_length        = 14
  require_lowercase_characters   = true
  require_numbers                = true
  require_uppercase_characters   = true
  require_symbols                = false
  allow_users_to_change_password = true
  max_password_age               = 45
}

resource "aws_iam_virtual_mfa_device" "sam-ca-mfa" {
  virtual_mfa_device_name = "sam-ca-mfa-device"
}

output "mfa_qr" {
  sensitive = true
  value     = aws_iam_virtual_mfa_device.sam-ca-mfa.qr_code_png
}

resource "aws_iam_user" "pearly-whites" {
  name = "casetoner"

  tags = {
    tag-key = "tf-managed-user"
  }
}

resource "aws_iam_access_key" "pearly-key" {
  user = aws_iam_user.pearly-whites.name
}

resource "aws_iam_user_policy" "pearly-policy" {
  name = "pearly-policy"
  user = aws_iam_user.pearly-whites.name

  # Terraform's "jsonencode" function converts a
  # Terraform expression result to valid JSON syntax.
  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Action = [
          "ec2:Describe*",
        ]
        Effect   = "Allow"
        Resource = "*"
      },
    ]
  })
}

resource "aws_iam_policy" "policy" {
  name        = "test-policy"
  description = "A test policy"
  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Action = [
          "s3:List*",
        ]
        Effect   = "Allow"
        Resource = "*"
      },
    ]
  })
}

resource "aws_iam_user_policy_attachment" "test-attach" {
  user       = aws_iam_user.pearly-whites.name
  policy_arn = aws_iam_policy.policy.arn
}


data "aws_iam_policy_document" "assume_role" {
  statement {
    effect = "Allow"

    principals {
      type        = "Service"
      identifiers = ["ec2.amazonaws.com"]
    }

    actions = ["sts:AssumeRole"]
  }
}

resource "aws_iam_role" "role" {
  name               = "security-test-role"
  assume_role_policy = data.aws_iam_policy_document.assume_role.json
}

resource "aws_iam_role_policy_attachment" "test-attach" {
  role       = aws_iam_role.role.name
  policy_arn = "arn:aws:iam::aws:policy/AWSSupportAccess"
}

resource "aws_iam_server_certificate" "test_cert" {
  name             = "some_test_cert"
  certificate_body = file("cert.pem")
  private_key      = file("key.pem")
}

resource "aws_accessanalyzer_analyzer" "account_analyzer" {
  analyzer_name = "account_analyzer"
}

resource "aws_cloudtrail" "test-cloudtrail" {
  depends_on = [aws_s3_bucket_policy.s3_bucket_policy_test]

  name                          = "cloudtrail_test"
  s3_bucket_name                = aws_s3_bucket.s3_bucket_cloudtrail_test.id
  s3_key_prefix                 = "prefix"
  include_global_service_events = true
  is_multi_region_trail         = true
  enable_logging                = true
  enable_log_file_validation    = true

  event_selector {
    read_write_type           = "All"
    include_management_events = true

    data_resource {
      type   = "AWS::S3::Object"
      values = ["arn:aws:s3"]
    }
  }
}

resource "aws_s3_bucket" "s3_bucket_cloudtrail_test" {
  bucket        = "tf-test-s3-trail-dcap"
  force_destroy = true
}

data "aws_iam_policy_document" "aws_iam_policy_doc_s3" {
  statement {
    sid    = "AWSCloudTrailAclCheck"
    effect = "Allow"

    principals {
      type        = "Service"
      identifiers = ["cloudtrail.amazonaws.com"]
    }

    actions   = ["s3:GetBucketAcl"]
    resources = [aws_s3_bucket.s3_bucket_cloudtrail_test.arn]
    condition {
      test     = "StringEquals"
      variable = "aws:SourceArn"
      values   = ["arn:${data.aws_partition.current.partition}:cloudtrail:${data.aws_region.current.name}:${data.aws_caller_identity.current.account_id}:trail/cloudtrail_test"]
    }
  }

  statement {
    sid    = "AWSDenyS3HTTPRequests_CIS2-1-1"
    effect = "Deny"

    principals {
      type        = "*"
      identifiers = ["*"]
    }

    actions   = ["s3:*"]
    resources = [aws_s3_bucket.s3_bucket_cloudtrail_test.arn]
    condition {
      test     = "Bool"
      variable = "aws:SecureTransport"
      values   = ["false"]
    }
  }

  statement {
    sid    = "AWSCloudTrailWrite"
    effect = "Allow"

    principals {
      type        = "Service"
      identifiers = ["cloudtrail.amazonaws.com"]
    }

    actions   = ["s3:PutObject"]
    resources = ["${aws_s3_bucket.s3_bucket_cloudtrail_test.arn}/prefix/AWSLogs/${data.aws_caller_identity.current.account_id}/*"]

    condition {
      test     = "StringEquals"
      variable = "s3:x-amz-acl"
      values   = ["bucket-owner-full-control"]
    }
    condition {
      test     = "StringEquals"
      variable = "aws:SourceArn"
      values   = ["arn:${data.aws_partition.current.partition}:cloudtrail:${data.aws_region.current.name}:${data.aws_caller_identity.current.account_id}:trail/cloudtrail_test"]
    }
  }
}

resource "aws_s3_bucket_policy" "s3_bucket_policy_test" {
  bucket = aws_s3_bucket.s3_bucket_cloudtrail_test.id
  policy = data.aws_iam_policy_document.aws_iam_policy_doc_s3.json
}

data "aws_caller_identity" "current" {}

data "aws_partition" "current" {}

data "aws_region" "current" {}


resource "aws_config_config_rule" "config_rule" {
  name = "config_rule"

  source {
    owner             = "AWS"
    source_identifier = "S3_BUCKET_VERSIONING_ENABLED"
  }

  depends_on = [aws_config_configuration_recorder.config_recorder]
}

resource "aws_config_configuration_recorder" "config_recorder" {
  name     = "config_recorder"
  role_arn = aws_iam_role.config_role.arn
  recording_group {
    all_supported = "true"
    include_global_resource_types = "true"

  }
}

resource "aws_config_configuration_recorder_status" "config_recorder_status" {
  name       = aws_config_configuration_recorder.config_recorder.name
  is_enabled = true
  ## Needs a "LastStatus=SUCCESS to pass. i.e. needs to run (CIS 3.5)"
}

data "aws_iam_policy_document" "assume_role" {
  statement {
    effect = "Allow"

    principals {
      type        = "Service"
      identifiers = ["config.amazonaws.com"]
    }

    actions = ["sts:AssumeRole"]
  }
}

resource "aws_iam_role" "config_role" {
  name               = "my-awsconfig-role"
  assume_role_policy = data.aws_iam_policy_document.assume_role.json
}

data "aws_iam_policy_document" "policy_document" {
  statement {
    effect    = "Allow"
    actions   = ["config:Put*"]
    resources = ["*"]
  }
}

resource "aws_iam_role_policy" "p" {
  name   = "my-awsconfig-policy"
  role   = aws_iam_role.config_role.id
  policy = data.aws_iam_policy_document.policy_document.json
}

# resource "aws_route53_zone" "primary" {
#   name = "example.com"
# }

# resource "aws_route53_record" "www" {
#   zone_id = aws_route53_zone.primary.zone_id
#   name    = "www.example.com"
#   type    = "A"
#   ttl     = 300
#   records = [aws_eip.lb.public_ip]
# }