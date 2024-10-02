terraform {
  required_providers {
    azurerm = {
      source  = "hashicorp/azurerm"
      version = "~>4.0"
    }
  }
  required_version = "~> 1.9.5"
}

locals {
  resource_group = "SSPHP"
  tags = {
    "Product"          = "Protective Monitoring - Splunk SaaS"
    "Environment"      = "Development"
    "Service Offering" = "Protective Monitoring - Splunk SaaS"
  }
  #  sku_name_python      = "Y1"
  sku_name_rust        = "EP1"
  key_vault_name       = "SSPHP-Metrics-i4tm3"
  key_vault_object_ids = ["3d088dc7-61ad-439d-82e4-0fe2b3874751"]
}

module "data_ingester" {
  source         = "../data_ingester"
  resource_group = local.resource_group
  #  sku_name_python      = local.sku_name_python
  sku_name_rust        = local.sku_name_rust
  key_vault_name       = local.key_vault_name
  key_vault_object_ids = local.key_vault_object_ids
  tags                 = local.tags
  vnet = {
    name        = "ssphp_test_vnet",
    subnet_name = "default"
  }
}
