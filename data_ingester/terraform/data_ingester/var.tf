variable "resource_group" {
  type = string
}

variable "tags" {
  type = map(string)
}

variable "sku_name_python" {
  type = string
}

variable "sku_name_rust" {
  type = string
}

variable "key_vault_name" {
  type = string
}

variable "key_vault_object_ids" {
  type = list(string)
}
