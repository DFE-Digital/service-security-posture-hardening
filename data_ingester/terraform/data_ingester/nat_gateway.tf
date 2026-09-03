data "azurerm_subnet" "integration" {
  count                = var.vnet == null || var.egress_via_nat_gateway ? 0 : 1
  name                 = var.vnet.subnet_name
  virtual_network_name = var.vnet.name
  resource_group_name  = var.vnet.resource_group_name
}

locals {
  provision_nat_gateway = var.egress_via_nat_gateway
  integration_subnet_id = local.provision_nat_gateway ? azurerm_subnet.egress[0].id : (
    var.vnet == null ? null : data.azurerm_subnet.integration[0].id
  )
}

resource "azurerm_virtual_network" "egress" {
  count               = local.provision_nat_gateway ? 1 : 0
  name                = "SSPHP-Metrics-rust-egress-vnet-${local.postfix}"
  resource_group_name = azurerm_resource_group.tfstate.name
  location            = azurerm_resource_group.tfstate.location
  address_space       = var.egress_vnet_address_space
  tags                = var.tags

  lifecycle {
    prevent_destroy = true
  }
}

resource "azurerm_subnet" "egress" {
  count                = local.provision_nat_gateway ? 1 : 0
  name                 = "function-integration"
  resource_group_name  = azurerm_resource_group.tfstate.name
  virtual_network_name = azurerm_virtual_network.egress[0].name
  address_prefixes     = var.egress_subnet_address_prefixes

  delegation {
    name = "function-app-integration"

    service_delegation {
      name = "Microsoft.Web/serverFarms"
      actions = [
        "Microsoft.Network/virtualNetworks/subnets/action",
      ]
    }
  }
}

resource "azurerm_public_ip" "egress" {
  count               = local.provision_nat_gateway ? 1 : 0
  name                = "SSPHP-Metrics-rust-egress-${local.postfix}"
  resource_group_name = azurerm_resource_group.tfstate.name
  location            = azurerm_resource_group.tfstate.location
  allocation_method   = "Static"
  sku                 = "Standard"
  zones               = ["1", "2", "3"]
  tags                = var.tags

  lifecycle {
    prevent_destroy = true
  }
}

resource "azurerm_nat_gateway" "egress" {
  count                   = local.provision_nat_gateway ? 1 : 0
  name                    = "SSPHP-Metrics-rust-natgw-${local.postfix}"
  resource_group_name     = azurerm_resource_group.tfstate.name
  location                = azurerm_resource_group.tfstate.location
  sku_name                = "Standard"
  idle_timeout_in_minutes = 10
  tags                    = var.tags
}

resource "azurerm_nat_gateway_public_ip_association" "egress" {
  count                = local.provision_nat_gateway ? 1 : 0
  nat_gateway_id       = azurerm_nat_gateway.egress[0].id
  public_ip_address_id = azurerm_public_ip.egress[0].id
}

resource "azurerm_subnet_nat_gateway_association" "egress" {
  count          = local.provision_nat_gateway ? 1 : 0
  subnet_id      = azurerm_subnet.egress[0].id
  nat_gateway_id = azurerm_nat_gateway.egress[0].id
}
