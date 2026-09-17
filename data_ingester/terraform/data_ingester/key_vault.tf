data "azurerm_client_config" "current" {}

resource "azurerm_key_vault" "SSPHP" {
  count                           = var.manage_key_vault ? 1 : 0
  name                            = var.key_vault_name
  location                        = azurerm_resource_group.tfstate.location
  resource_group_name             = azurerm_resource_group.tfstate.name
  tags                            = var.tags
  tenant_id                       = data.azurerm_client_config.current.tenant_id
  soft_delete_retention_days      = 7
  purge_protection_enabled        = true
  enabled_for_template_deployment = true
  sku_name                        = "standard"

  lifecycle {
    prevent_destroy = true
  }

}

data "azurerm_key_vault" "shared" {
  count               = var.manage_key_vault ? 0 : 1
  name                = var.key_vault_name
  resource_group_name = var.shared_key_vault_resource_group
}

locals {
  key_vault_id = var.manage_key_vault ? azurerm_key_vault.SSPHP[0].id : data.azurerm_key_vault.shared[0].id
}

resource "azurerm_key_vault_access_policy" "platform" {
  for_each = var.manage_key_vault ? toset(var.key_vault_object_ids) : toset([])

  key_vault_id = local.key_vault_id
  tenant_id    = data.azurerm_client_config.current.tenant_id
  object_id    = each.value

  key_permissions = ["Get", "List"]
  secret_permissions = [
    "Get",
    "List",
    "Set",
  ]
  storage_permissions = [
    "Get",
    "List",
    "Set",
  ]
  certificate_permissions = [
    "Get",
    "List",
    "Create",
    "Delete",
  ]
}

resource "azurerm_key_vault_access_policy" "function" {
  key_vault_id = local.key_vault_id
  tenant_id    = azurerm_linux_function_app.SSPHP_rust.identity[0].tenant_id
  object_id    = azurerm_linux_function_app.SSPHP_rust.identity[0].principal_id

  key_permissions = ["Get", "List"]
  secret_permissions = [
    "Get",
    "List",
  ]
  storage_permissions = [
    "Get",
    "List",
  ]
  certificate_permissions = [
    "Get",
    "List",
  ]
}

resource "azurerm_key_vault_certificate" "example" {
  count        = var.manage_key_vault ? 1 : 0
  name         = "ad-client-certificate"
  key_vault_id = local.key_vault_id

  certificate_policy {
    issuer_parameters {
      name = "Self"
    }

    key_properties {
      exportable = true
      key_size   = 2048
      key_type   = "RSA"
      reuse_key  = true
    }

    lifetime_action {
      action {
        action_type = "AutoRenew"
      }

      trigger {
        days_before_expiry = 30
      }
    }

    secret_properties {
      content_type = "application/x-pkcs12"
    }

    x509_certificate_properties {
      key_usage = [
        "cRLSign",
        "dataEncipherment",
        "digitalSignature",
        "keyAgreement",
        "keyCertSign",
        "keyEncipherment",
      ]

      subject_alternative_names {
        dns_names = ["data-ingester.ssphp.education.gov.uk"]
      }

      subject            = "CN=data-ingester.ssphp.education.gov.uk"
      validity_in_months = 12
    }
  }

  lifecycle {
    prevent_destroy = true
  }

}
