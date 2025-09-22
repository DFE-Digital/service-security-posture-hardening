resource "azurerm_service_plan" "SSPHP_rust" {
  name                = "SSPHP-Metrics_rust-${local.postfix}"
  resource_group_name = azurerm_resource_group.tfstate.name
  location            = azurerm_resource_group.tfstate.location
  os_type             = "Linux"
  sku_name            = var.sku_name_rust
  tags                = var.tags
}

data "archive_file" "data_ingester_rust" {
  type        = "zip"
  source_dir  = "${path.module}/../../data_ingester_workspace/data_ingester_axum_entrypoint/function_zip/"
  output_path = "${path.module}/data_ingester_rust_${formatdate("YYYYMMDDHHmmss", timestamp())}.zip"
}

resource "azurerm_linux_function_app" "SSPHP_rust" {
  name                       = "SSPHP-Metrics-rust-${local.postfix}"
  resource_group_name        = azurerm_resource_group.tfstate.name
  location                   = azurerm_resource_group.tfstate.location
  tags                       = var.tags
  storage_account_name       = azurerm_storage_account.tfstate.name
  storage_account_access_key = azurerm_storage_account.tfstate.primary_access_key
  service_plan_id            = azurerm_service_plan.SSPHP_rust.id
  enabled                    = true
  builtin_logging_enabled    = false
  https_only                 = true

  identity {
    type = "SystemAssigned"
  }

  site_config {
    minimum_tls_version        = "1.2"
    cors {
      allowed_origins     = ["https://portal.azure.com", ]
      support_credentials = true
    }
    application_stack {
      use_custom_runtime = true
    }
  }

  zip_deploy_file = data.archive_file.data_ingester_rust.output_path
  # https://learn.microsoft.com/en-us/azure/azure-functions/functions-app-settings
  app_settings = {
    WEBSITE_RUN_FROM_PACKAGE    = "1"
    KEY_VAULT_NAME              = var.key_vault_name
    RUST_BACKTRACE              = "1"
    RUST_LOG                    = "info"
    FUNCTIONS_EXTENSION_VERSION = "~4"
    # linuxFxVersion Value taken from this command
    # az functionapp list-runtimes --os linux --query "[].{stack:join(' ', [runtime, version]), LinuxFxVersion:linux_fx_version, SupportedFunctionsVersions:to_string(supported_functions_versions[])}" --output table
    linuxFxVersion = ""
    # https://github.com/Azure/azure-functions-host/issues/8021
    alwaysOn = true
  }
}
