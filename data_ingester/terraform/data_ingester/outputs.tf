output "egress_ip" {
  description = "Static public IP used for outbound traffic when the function app is deployed with VNet integration AND a dedicated NAT Gateway (`var.egress_via_nat_gateway = true`). Null when either `var.vnet` is null or the environment egresses via the integration subnet's existing routing (e.g. a platform NVA/firewall) — in that case the egress IP is owned by the platform team, not this module."
  value       = local.provision_nat_gateway ? azurerm_public_ip.egress[0].ip_address : null
}
