variable "resource_group" {
  description = "Name of the resources group to deploy into"
  type        = string
}

variable "tags" {
  description = "Tags to add to resources"
  type        = map(string)
}

variable "sku_name_rust" {
  description = "The function app SKU to run the function"
  type        = string
}

variable "key_vault_name" {
  description = "The name of the keyvault"
  type        = string
}

variable "key_vault_object_ids" {
  description = "Additional IDs to add into the keyvault access policies"
  type        = list(string)
}

variable "vnet" {
  description = "Deploy the function into an existing VNET. `name` is the name of the vnet, `subnet_name` is the name of the subnet"
  type = object({
    name                = string,
    subnet_name         = string,
    resource_group_name = string,
  })
  default = null
}

variable "egress_via_nat_gateway" {
  description = "When true, provision a dedicated VNet, delegated Function App integration subnet, NAT Gateway, and Standard Static Public IP for internet egress."
  type        = bool
  default     = false
}

variable "egress_vnet_address_space" {
  description = "Address space for the dedicated egress VNet. It must not overlap with connected networks."
  type        = list(string)
  default     = ["10.250.0.0/16"]
}

variable "egress_subnet_address_prefixes" {
  description = "Address prefixes for the delegated Function App integration subnet."
  type        = list(string)
  default     = ["10.250.0.0/24"]
}

variable "random_postfix" {
  description = "Override the random string postfixed to resource names"
  type        = string
  default     = null
}
