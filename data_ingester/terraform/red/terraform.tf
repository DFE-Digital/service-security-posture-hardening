terraform {
  required_providers {
    azurerm = {
      source  = "hashicorp/azurerm"
      version = "~>4.0"
    }
  }

  backend "azurerm" {
    resource_group_name  = "s194d00-SSPHP-Metrics"
    storage_account_name = "tfstatep3sha"
    container_name       = "tfstate"
    key                  = "terraform.tfstate"
  }
}

locals {
  resource_group = "s194d00-SSPHP-Metrics"
  tags = {
    "Product"          = "Protective Monitoring - Splunk SaaS"
    "Environment"      = "Dev"
    "Service Offering" = "Protective Monitoring - Splunk SaaS"

  }
  #  sku_name_python      = "Y1"
  sku_name_rust  = "EP1"
  key_vault_name = "SSPHP-Metrics"
  key_vault_object_ids = [
    "393279ef-dc89-4bff-8186-4d283ee7b280",
    "7e53a6ad-8c94-4d82-a3b5-222882dffc29",
    "0870054a-44a7-4ddf-af8d-e48276f81df4",
    "14b985a0-2abe-4844-b1ac-82ca64943724",
  ] # 7e53 = GitHub Actions - TODO; check if still required
}

provider "azurerm" {
  features {}
}

module "data_ingester" {
  source         = "../data_ingester"
  resource_group = local.resource_group
  #  sku_name_python      = local.sku_name_python
  sku_name_rust        = local.sku_name_rust
  key_vault_name       = local.key_vault_name
  key_vault_object_ids = local.key_vault_object_ids
  tags                 = local.tags
}
