##############################################################################
# GuardLinkX Storefront — Terraform Outputs
##############################################################################

output "public_ip" {
  description = "Elastic IP address — set this as an A record in IONOS DNS"
  value       = aws_eip.guardlinkx.public_ip
}

output "ssh_command" {
  description = "SSH command to connect to the server"
  value       = "ssh -i <your-key.pem> ubuntu@${aws_eip.guardlinkx.public_ip}"
}

output "website_url" {
  description = "Website URL (HTTPS will work after Certbot setup)"
  value       = "https://${var.domain_name}"
}

output "instance_id" {
  description = "EC2 instance ID"
  value       = aws_instance.guardlinkx.id
}
