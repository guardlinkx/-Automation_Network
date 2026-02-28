output "vpn_us_east_ip" {
  description = "Public IP of the US East VPN node"
  value       = module.vpn_us_east.public_ip
}

output "vpn_us_east_ssh" {
  description = "SSH command for US East VPN node"
  value       = "ssh -i ${var.key_name}.pem ubuntu@${module.vpn_us_east.public_ip}"
}

# Phase 2 outputs (uncomment when expanding)
# output "vpn_eu_central_ip" {
#   value = module.vpn_eu_central.public_ip
# }
# output "vpn_ap_northeast_ip" {
#   value = module.vpn_ap_northeast.public_ip
# }
# output "vpn_eu_west_ip" {
#   value = module.vpn_eu_west.public_ip
# }
# output "vpn_ca_central_ip" {
#   value = module.vpn_ca_central.public_ip
# }
