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

variable "random_postfix" {
  description = "Override the random string postfixed to resource names"
  type        = string
  default     = null
}
