terraform {
  required_providers {
    azurerm = {
      source  = "hashicorp/azurerm"
      version = "~>3.0"
    }
  }

    backend "azurerm" {
      resource_group_name  = "s194d00-SSPHP-Metrics"
      storage_account_name = "tfstatel95cd"
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
    "Product"     = "Protective Monitoring - Splunk SaaS"
    "Environment" = "Development"
    "Service Offering" = "Protective Monitoring - Splunk SaaS"

  }
}

resource "azurerm_storage_account" "tfstate" {
  name                            = "tfstate${random_string.resource_code.result}"
  resource_group_name             = azurerm_resource_group.tfstate.name
  location                        = azurerm_resource_group.tfstate.location
  account_tier                    = "Standard"
  account_replication_type        = "LRS"
  allow_nested_items_to_be_public = false
}

resource "azurerm_storage_container" "tfstate" {
  name                  = "tfstate"
  storage_account_name  = azurerm_storage_account.tfstate.name
  container_access_type = "private"
}
