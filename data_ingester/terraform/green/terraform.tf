terraform {
  required_providers {
    azurerm = {
      source  = "hashicorp/azurerm"
      version = "~>4.0"
    }
  }

  required_version = "~> 1.9.5"

  backend "azurerm" {
    resource_group_name  = "s194d00-SSPHP-Metrics"
    storage_account_name = "tfstatep3sha"
    container_name       = "tfstate"
    key                  = "green.tfstate"
  }
}

locals {
  resource_group = "s194d00-SSPHP-Metrics-green"
  tags = {
    "Product"          = "Protective Monitoring - Splunk SaaS"
    "Environment"      = "Dev"
    "Service Offering" = "Protective Monitoring - Splunk SaaS"
  }
  sku_name_rust                   = "EP1"
  key_vault_name                  = "SSPHP-Metrics"
  shared_key_vault_resource_group = "s194d00-SSPHP-Metrics"
  key_vault_object_ids            = []
}

provider "azurerm" {
  features {}
}

module "data_ingester" {
  source                          = "../data_ingester"
  resource_group                  = local.resource_group
  sku_name_rust                   = local.sku_name_rust
  key_vault_name                  = local.key_vault_name
  key_vault_object_ids            = local.key_vault_object_ids
  manage_key_vault                = false
  shared_key_vault_resource_group = local.shared_key_vault_resource_group
  random_postfix                  = "green"
  egress_via_nat_gateway          = true
  egress_vnet_address_space       = ["10.251.0.0/16"]
  egress_subnet_address_prefixes  = ["10.251.0.0/24"]
  tags                            = local.tags
}

output "egress_ip" {
  description = "Static public IP used for green outbound traffic."
  value       = module.data_ingester.egress_ip
}
