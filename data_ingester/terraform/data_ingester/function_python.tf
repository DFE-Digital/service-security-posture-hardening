resource "azurerm_service_plan" "SSPHP" {
  name                = "SSPHP-Metrics-${random_string.resource_code.result}"
  resource_group_name = azurerm_resource_group.tfstate.name
  location            = azurerm_resource_group.tfstate.location
  os_type             = "Linux"
  sku_name            = var.sku_name_python
  tags                = var.tags
}

data "archive_file" "deployment" {
  type        = "zip"
  source_dir  = "${path.module}/../../data_ingester/"
  output_path = "${path.module}/deployment_${formatdate("YYYYMMDDHHmmss", timestamp())}.zip"
}

resource "azurerm_linux_function_app" "SSPHP" {
  name                       = "SSPHP-Metrics-${random_string.resource_code.result}"
  resource_group_name        = azurerm_resource_group.tfstate.name
  location                   = azurerm_resource_group.tfstate.location
  tags                       = var.tags
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
    KEY_VAULT_NAME                 = var.key_vault_name
  }
}
