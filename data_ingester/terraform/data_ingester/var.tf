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
  description = "When true, provision a dedicated NAT Gateway + Standard Static Public IP and attach it to the integration subnet so the function app has a tenant-exclusive outbound IP. Leave false (default) to egress via the integration subnet's existing routing (e.g. a platform-owned NVA/firewall). Only takes effect when `var.vnet` is set. Note: if the integration subnet already has a UDR forcing 0.0.0.0/0 to a virtual appliance, that UDR wins over any subnet-attached NAT Gateway, so setting this to true in that case would provision an unused resource."
  type        = bool
  default     = false
}

variable "random_postfix" {
  description = "Override the random string postfixed to resource names"
  type        = string
  default     = null
}
