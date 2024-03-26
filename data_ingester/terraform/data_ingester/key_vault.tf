data "azurerm_client_config" "current" {}

# locals {
#   key_vault_name = "${var.key_vault_name}-${random_string.resource_code.result}"
# }


resource "azurerm_key_vault" "SSPHP" {
  name                            = var.key_vault_name
  location                        = azurerm_resource_group.tfstate.location
  resource_group_name             = azurerm_resource_group.tfstate.name
  tags                            = var.tags
  tenant_id                       = data.azurerm_client_config.current.tenant_id
  soft_delete_retention_days      = 7
  purge_protection_enabled        = true
  enabled_for_template_deployment = true
  sku_name                        = "standard"


  dynamic "access_policy" {
    for_each = toset(var.key_vault_object_ids)
    content {
      tenant_id = data.azurerm_client_config.current.tenant_id
      object_id = access_policy.value

      key_permissions = [
        "Get",
        "List"
      ]

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
  }

  # access_policy {
  #   tenant_id = azurerm_linux_function_app.SSPHP.identity[0].tenant_id
  #   object_id = azurerm_linux_function_app.SSPHP.identity[0].principal_id

  #   key_permissions = [
  #     "Get",
  #     "List"
  #   ]

  #   secret_permissions = [
  #     "Get",
  #     "List",
  #   ]

  #   storage_permissions = [
  #     "Get",
  #     "List",
  #   ]

  # }

  access_policy {
    tenant_id = azurerm_linux_function_app.SSPHP_rust.identity[0].tenant_id
    object_id = azurerm_linux_function_app.SSPHP_rust.identity[0].principal_id

    key_permissions = [
      "Get",
      "List"
    ]

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
}

resource "azurerm_key_vault_certificate" "example" {

  name         = "ad-client-certificate"
  key_vault_id = azurerm_key_vault.SSPHP.id

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
}
