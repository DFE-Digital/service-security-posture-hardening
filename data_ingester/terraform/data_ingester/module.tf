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
  name     = var.resource_group
  location = "West Europe"
  tags     = var.tags
  # {
  #   "Product"          = "Protective Monitoring - Splunk SaaS"
  #   "Environment"      = "Development"
  #   "Service Offering" = "Protective Monitoring - Splunk SaaS"
  # }
}

resource "azurerm_storage_account" "tfstate" {
  name                            = "tfstate${random_string.resource_code.result}"
  resource_group_name             = azurerm_resource_group.tfstate.name
  location                        = azurerm_resource_group.tfstate.location
  account_tier                    = "Standard"
  account_replication_type        = "LRS"
  allow_nested_items_to_be_public = false
  tags                            = var.tags
}

resource "azurerm_storage_container" "tfstate" {
  name                  = "tfstate"
  storage_account_name  = azurerm_storage_account.tfstate.name
  container_access_type = "private"
}

resource "azurerm_application_insights" "SSPHP" {
  name                = "SSPHP-Metrics"
  location            = azurerm_resource_group.tfstate.location
  resource_group_name = azurerm_resource_group.tfstate.name
  application_type    = "other"
  tags                = var.tags
}
