terraform {
  required_providers {
    azurerm = {
      source  = "hashicorp/azurerm"
      version = "~>3.0"
    }
  }

  backend "azurerm" {
    resource_group_name  = "s194d00-SSPHP-Metrics"
    storage_account_name = "tfstatep3sha"
    container_name       = "tfstate"
    key                  = "terraform.tfstate"
  }
}

provider "azurerm" {
  skip_provider_registration = true
  features {}
}

resource "random_string" "resource_code" {
  length  = 5
  special = false
  upper   = false
}

resource "azurerm_resource_group" "tfstate" {
  name     = "s194d00-SSPHP-Metrics"
  location = "West Europe"
  tags = {
    "Product"          = "Protective Monitoring - Splunk SaaS"
    "Environment"      = "Development"
    "Service Offering" = "Protective Monitoring - Splunk SaaS"

  }
}

resource "azurerm_storage_account" "tfstate" {
  name                = "tfstate${random_string.resource_code.result}"
  resource_group_name = azurerm_resource_group.tfstate.name
  location            = azurerm_resource_group.tfstate.location
  account_tier        = "Standard"
  #infrastructure_encryption_enabled = true
  account_replication_type        = "LRS"
  allow_nested_items_to_be_public = false
}

resource "azurerm_storage_container" "tfstate" {
  name                  = "tfstate"
  storage_account_name  = azurerm_storage_account.tfstate.name
  container_access_type = "private"
}

resource "azurerm_service_plan" "SSPHP" {
  name                = "SSPHP-Metrics"
  resource_group_name = azurerm_resource_group.tfstate.name
  location            = azurerm_resource_group.tfstate.location
  os_type             = "Linux"
  sku_name            = "Y1"
}

# resource "null_resource" "always_run" {
#   triggers = {
#     timestamp = "${formatdate("YYYYMMDDHHmmss", timestamp())}"
#   }
# }

data "archive_file" "deployment" {
  type        = "zip"
  source_dir  = "${path.module}/../../data_ingester/"
  output_path = "${path.module}/deployment_${formatdate("YYYYMMDDHHmmss", timestamp())}.zip"
}

resource "azurerm_linux_function_app" "SSPHP" {
  name                = "SSPHP-Metrics"
  resource_group_name = azurerm_resource_group.tfstate.name
  location            = azurerm_resource_group.tfstate.location

  storage_account_name       = azurerm_storage_account.tfstate.name
  storage_account_access_key = azurerm_storage_account.tfstate.primary_access_key
  service_plan_id            = azurerm_service_plan.SSPHP.id
  enabled                    = true
  builtin_logging_enabled    = true

  identity {
    type = "SystemAssigned"
  }

  site_config {
    application_stack {
      python_version = "3.10"
    }
    cors {
      allowed_origins     = ["https://portal.azure.com", ]
      support_credentials = true
    }
  }
  zip_deploy_file = data.archive_file.deployment.output_path
  app_settings = {
    WEBSITE_RUN_FROM_PACKAGE       = "1"
    APPINSIGHTS_INSTRUMENTATIONKEY = azurerm_application_insights.SSPHP.instrumentation_key
    KEY_VAULT_NAME                 = local.key_vault_name
  }
  # lifecycle {
  #   replace_triggered_by = [
  #     null_resource.always_run
  #   ]
  # }
}

resource "azurerm_service_plan" "SSPHP_rust" {
  name                = "SSPHP-Metrics_rust-${random_string.resource_code.result}"
  resource_group_name = azurerm_resource_group.tfstate.name
  location            = azurerm_resource_group.tfstate.location
  os_type             = "Linux"
  sku_name            = "EP1"
}

data "archive_file" "data_ingester_rust" {
  type        = "zip"
  source_dir  = "${path.module}/../../data_ingester_rust/function_zip/"
  output_path = "${path.module}/data_ingester_rust_${formatdate("YYYYMMDDHHmmss", timestamp())}.zip"
}

resource "azurerm_linux_function_app" "SSPHP_rust" {
  name                = "SSPHP-Metrics-rust-${random_string.resource_code.result}"
  resource_group_name = azurerm_resource_group.tfstate.name
  location            = azurerm_resource_group.tfstate.location

  storage_account_name       = azurerm_storage_account.tfstate.name
  storage_account_access_key = azurerm_storage_account.tfstate.primary_access_key
  service_plan_id            = azurerm_service_plan.SSPHP_rust.id
  enabled                    = true
  builtin_logging_enabled    = true

  identity {
    type = "SystemAssigned"
  }

  site_config {
    cors {
      allowed_origins     = ["https://portal.azure.com", ]
      support_credentials = true
    }
    elastic_instance_minimum = 1
  }

  zip_deploy_file = data.archive_file.data_ingester_rust.output_path
  app_settings = {
    WEBSITE_RUN_FROM_PACKAGE       = "1"
    APPINSIGHTS_INSTRUMENTATIONKEY = azurerm_application_insights.SSPHP.instrumentation_key
    KEY_VAULT_NAME                 = local.key_vault_name
  }
}

locals {
  key_vault_name = "SSPHP-Metrics"
}

resource "azurerm_application_insights" "SSPHP" {
  name                = "SSPHP-Metrics"
  location            = azurerm_resource_group.tfstate.location
  resource_group_name = azurerm_resource_group.tfstate.name
  application_type    = "other"
}

data "azurerm_client_config" "current" {}

resource "azurerm_key_vault" "SSPHP" {
  name                       = local.key_vault_name
  location                   = azurerm_resource_group.tfstate.location
  resource_group_name        = azurerm_resource_group.tfstate.name
  tenant_id                  = data.azurerm_client_config.current.tenant_id
  soft_delete_retention_days = 7
  purge_protection_enabled   = false


  enabled_for_template_deployment = true
  sku_name                        = "standard"

  # access_policy {
  #   tenant_id = data.azurerm_client_config.current.tenant_id
  #   object_id = "63e06276-f3cb-4b5a-a23e-669869e8ef2a" # data.azurerm_client_config.current.object_id

  #   key_permissions = [
  #     "Get",
  #   ]

  #   secret_permissions = [
  #     "Get",
  #   ]

  #   storage_permissions = [
  #     "Get",
  #   ]
  # }

  access_policy {
    tenant_id = data.azurerm_client_config.current.tenant_id
    object_id = "393279ef-dc89-4bff-8186-4d283ee7b280"

    key_permissions = [
      "Get",
      "List"
    ]

    secret_permissions = [
      "Get",
      "List",
      "Set",
    ]

    storage_permissions = [
      "Get",
      "List",
      "Set",
    ]
  }

  access_policy {
    tenant_id = azurerm_linux_function_app.SSPHP.identity[0].tenant_id
    object_id = azurerm_linux_function_app.SSPHP.identity[0].principal_id

    key_permissions = [
      "Get",
      "List"
    ]

    secret_permissions = [
      "Get",
      "List",
    ]

    storage_permissions = [
      "Get",
      "List",
    ]
  }

  access_policy {
    tenant_id = azurerm_linux_function_app.SSPHP_rust.identity[0].tenant_id
    object_id = azurerm_linux_function_app.SSPHP_rust.identity[0].principal_id

    key_permissions = [
      "Get",
      "List"
    ]

    secret_permissions = [
      "Get",
      "List",
    ]

    storage_permissions = [
      "Get",
      "List",
    ]
  }
}
