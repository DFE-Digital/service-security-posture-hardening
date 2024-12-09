terraform {
  required_providers {
    github = {
      source  = "integrations/github"
      version = "6.2.3"
    }
  }

  required_version = "~> 1.9.5"

}

provider "github" {
}

resource "random_string" "resource_code" {
  length  = 5
  special = false
  upper   = false
}


resource "github_repository" "fail" {
  name        = "test_repo_should_fail_${random_string.resource_code.result}"
  description = "This should fail all tests"

  visibility = "private"
}


resource "github_repository" "pass_main_branch_protection" {
  name        = "test_should_pass_main_branch_protection_${random_string.resource_code.result}"
  description = "test_should_pass_main_branch_protection"

  visibility = "public"

  auto_init = true

  security_and_analysis {
    secret_scanning {
      status = "enabled"
    }
  }

  vulnerability_alerts = true
}

resource "github_repository_dependabot_security_updates" "dependabot" {
  repository = github_repository.pass_main_branch_protection.name
  enabled    = true
}

resource "github_repository_file" "security_md" {
  repository          = github_repository.pass_main_branch_protection.name
  branch              = "main"
  file                = "SECURITY.md"
  content             = "#Security.md"
  commit_message      = "Managed by Terraform"
  commit_author       = "Terraform User"
  commit_email        = "terraform@example.com"
  overwrite_on_create = true
}

resource "github_branch_protection" "main_branch" {
  depends_on = [
    github_repository_file.security_md
  ]

  repository_id = github_repository.pass_main_branch_protection.node_id

  pattern                         = "main"
  enforce_admins                  = true
  allows_deletions                = false
  require_conversation_resolution = true
  require_signed_commits          = true

  required_status_checks {
    strict   = true
    contexts = ["ci/travis"]
  }

  required_pull_request_reviews {
    dismiss_stale_reviews           = true
    restrict_dismissals             = true
    require_code_owner_reviews      = true
    required_approving_review_count = 2
  }

}

data "github_user" "gh_user" {
  username = var.github_username
}

variable "github_username" {
  type    = string
  default = "akinnane"
}
