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
  builtin_logging_enabled    = true

  identity {
    type = "SystemAssigned"
  }

  site_config {
    cors {
      allowed_origins     = ["https://portal.azure.com", ]
      support_credentials = true
    }
    application_stack {
      use_custom_runtime = true
    }
  }

  zip_deploy_file = data.archive_file.data_ingester_rust.output_path
  app_settings = {
    WEBSITE_RUN_FROM_PACKAGE       = "1"
    APPINSIGHTS_INSTRUMENTATIONKEY = azurerm_application_insights.SSPHP.instrumentation_key
    KEY_VAULT_NAME                 = var.key_vault_name
    RUST_BACKTRACE                 = "1"
    RUST_LOG                       = "info"
  }
}

# Function in VNET
resource "azurerm_linux_function_app" "SSPHP_rust-vnet" {
  count                      = var.vnet != null ? 1 : 0
  name                       = "SSPHP-Metrics-rust-vnet-${local.postfix}"
  resource_group_name        = azurerm_resource_group.tfstate.name
  location                   = azurerm_resource_group.tfstate.location
  tags                       = var.tags
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
    application_stack {
      use_custom_runtime = true
    }
  }

  zip_deploy_file = data.archive_file.data_ingester_rust.output_path
  app_settings = {
    WEBSITE_RUN_FROM_PACKAGE       = "1"
    APPINSIGHTS_INSTRUMENTATIONKEY = azurerm_application_insights.SSPHP.instrumentation_key
    KEY_VAULT_NAME                 = var.key_vault_name
    RUST_BACKTRACE                 = "1"
    RUST_LOG                       = "info"
  }

  lifecycle {
    ignore_changes = [virtual_network_subnet_id]
  }
}

# Vnet join
resource "azurerm_app_service_virtual_network_swift_connection" "sspph_rust_vnet" {
  count          = var.vnet != null ? 1 : 0
  app_service_id = azurerm_linux_function_app.SSPHP_rust-vnet[0].id
  subnet_id      = data.azurerm_subnet.subnets[0].id
}
