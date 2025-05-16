resource "azurerm_resource_group" "dcap" {
  name     = "ca_test_group"
  location = "UK South"
}

resource "azurerm_kubernetes_cluster" "dcap" {
  name                = "ca_test"
  location            = azurerm_resource_group.dcap.location
  resource_group_name = azurerm_resource_group.dcap.name
  dns_prefix          = "ca-cluster"

  default_node_pool {
    name       = "default"
    node_count = 1
    vm_size    = "Standard_D2_v2"
  }

  identity {
    type = "SystemAssigned"
  }

  tags = {
    Environment = "development"
  }
}

resource "random_string" "acr_name" {
  length  = 5
  lower   = true
  numeric = false
  special = false
  upper   = false
}

resource "azurerm_container_registry" "dcap" {
  name                = "${random_string.acr_name.result}registry"
  resource_group_name = azurerm_resource_group.dcap.name
  location            = azurerm_resource_group.dcap.location
  sku                 = "Standard"
}



output "client_certificate" {
  value     = azurerm_kubernetes_cluster.dcap.kube_config[0].client_certificate
  sensitive = true
}

output "kube_config" {
  value = azurerm_kubernetes_cluster.dcap.kube_config_raw

  sensitive = true
}

output "container_registry_name" {
  value = azurerm_container_registry.dcap.name
}



# We strongly recommend using the required_providers block to set the
# Azure Provider source and version being used
terraform {
  required_providers {
    azurerm = {
      source  = "hashicorp/azurerm"
      version = "=4.1.0"
    }
  }
}

# Configure the Microsoft Azure Provider
provider "azurerm" {
  features {}

  subscription_id = "63ed7111-101c-4849-9f33-03ef672ed20d"
}



# add the role to the identity the kubernetes cluster was assigned
resource "azurerm_role_assignment" "kubweb_to_acr" {
  scope                = azurerm_container_registry.dcap.id
  role_definition_name = "AcrPull"
  principal_id         = azurerm_kubernetes_cluster.dcap.kubelet_identity[0].object_id
}
