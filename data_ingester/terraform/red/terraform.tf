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

locals {
  resource_group = "s194d00-SSPHP-Metrics"
  tags = {
    "Product"          = "Protective Monitoring - Splunk SaaS"
    "Environment"      = "Dev"
    "Service Offering" = "Protective Monitoring - Splunk SaaS"

  }
  sku_name_python      = "Y1"
  sku_name_rust        = "EP1"
  key_vault_name       = "SSPHP-Metrics"
  key_vault_object_ids = ["393279ef-dc89-4bff-8186-4d283ee7b280"]
}

provider "azurerm" {
  features {}
}

module "data_ingester" {
  source               = "../data_ingester"
  resource_group       = local.resource_group
  sku_name_python      = local.sku_name_python
  sku_name_rust        = local.sku_name_rust
  key_vault_name       = local.key_vault_name
  key_vault_object_ids = local.key_vault_object_ids
  tags                 = local.tags
}

moved {
  from = azurerm_resource_group.tfstate
  to   = module.data_ingester.azurerm_resource_group.tfstate
}

moved {
  from = azurerm_key_vault.SSPHP
  to   = module.data_ingester.azurerm_key_vault.SSPHP
}

moved {
  from = azurerm_service_plan.SSPHP
  to   = module.data_ingester.azurerm_service_plan.SSPHP
}

moved {
  from = azurerm_service_plan.SSPHP
  to   = module.data_ingester.azurerm_service_plan.SSPHP
}

moved {
  from = azurerm_storage_account.tfstate
  to   = module.data_ingester.azurerm_storage_account.tfstate
}

moved {
  from = azurerm_storage_container.tfstate
  to   = module.data_ingester.azurerm_storage_container.tfstate
}

moved {
  from = random_string.resource_code
  to   = module.data_ingester.random_string.resource_code
}


moved {
  from = azurerm_service_plan.SSPHP_rust
  to   = module.data_ingester.azurerm_service_plan.SSPHP_rust
}


moved {
  from = azurerm_application_insights.SSPHP
  to   = module.data_ingester.azurerm_application_insights.SSPHP
}

moved {
  from = azurerm_linux_function_app.SSPHP
  to   = module.data_ingester.azurerm_linux_function_app.SSPHP
}

moved {
  from = azurerm_linux_function_app.SSPHP_rust
  to   = module.data_ingester.azurerm_linux_function_app.SSPHP_rust
}
# moved {
#   from = 
#   to   = 
# }
# moved {
#   from = 
#   to   = 
# }
# moved {
#   from = 
#   to   = 
# }
