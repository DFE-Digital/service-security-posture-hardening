data "azurerm_virtual_network" "vnet" {
  count               = var.vnet != null ? 1 : 0
  name                = var.vnet.name
  resource_group_name = var.vnet.resource_group_name
}

data "azurerm_subnet" "subnets" {
  count                = var.vnet != null ? 1 : 0
  name                 = var.vnet.subnet_name
  resource_group_name  = var.vnet.resource_group_name
  virtual_network_name = data.azurerm_virtual_network.vnet[0].name
}

