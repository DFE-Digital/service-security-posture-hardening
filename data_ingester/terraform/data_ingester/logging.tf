resource "azurerm_log_analytics_workspace" "data_ingester" {
  name                = "SSPHP-Metrics-law-${local.postfix}"
  location            = azurerm_resource_group.tfstate.location
  resource_group_name = azurerm_resource_group.tfstate.name
  sku                 = "PerGB2018"
  retention_in_days   = 30
  tags                = var.tags
}

data "azurerm_monitor_diagnostic_categories" "data_ingester_function" {
  resource_id = azurerm_linux_function_app.SSPHP_rust.id
}

resource "azurerm_monitor_diagnostic_setting" "data_ingester_function" {
  name                       = "send-to-log-analytics"
  target_resource_id         = azurerm_linux_function_app.SSPHP_rust.id
  log_analytics_workspace_id = azurerm_log_analytics_workspace.data_ingester.id

  dynamic "enabled_log" {
    for_each = toset(data.azurerm_monitor_diagnostic_categories.data_ingester_function.log_category_types)
    content {
      category = enabled_log.value
    }
  }

  dynamic "metric" {
    for_each = toset(data.azurerm_monitor_diagnostic_categories.data_ingester_function.metrics)
    content {
      category = metric.value
      enabled  = true
    }
  }
}
