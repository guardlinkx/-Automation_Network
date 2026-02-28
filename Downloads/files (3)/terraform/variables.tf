##############################################################################
# GuardLinkX Storefront — Terraform Variables
##############################################################################

variable "aws_region" {
  description = "AWS region for all resources"
  type        = string
  default     = "us-east-1"
}

variable "instance_type" {
  description = "EC2 instance type"
  type        = string
  default     = "t3.medium"
}

variable "key_name" {
  description = "Name of an existing AWS EC2 key pair"
  type        = string
  default     = "guardlinkx-vpn-key"
}

variable "allowed_ssh_cidr" {
  description = "CIDR block allowed to SSH (restrict to your IP, e.g. 1.2.3.4/32)"
  type        = string
  default     = "0.0.0.0/0"
}

variable "domain_name" {
  description = "Fully qualified domain name for the storefront"
  type        = string
  default     = "califaxvpn.guardlinkx.com"
}

variable "admin_email" {
  description = "Admin email (used for Certbot and GuardLinkX admin login)"
  type        = string
  default     = "admin@guardlinkx.com"
}

variable "db_password" {
  description = "PostgreSQL password for the guardlinkx database user"
  type        = string
}

variable "flask_secret_key" {
  description = "Flask SECRET_KEY for session signing"
  type        = string
}

variable "stripe_secret_key" {
  description = "Stripe secret API key"
  type        = string
}

variable "stripe_publishable_key" {
  description = "Stripe publishable API key"
  type        = string
}

variable "stripe_webhook_secret" {
  description = "Stripe webhook signing secret"
  type        = string
}

variable "admin_password" {
  description = "GuardLinkX admin dashboard password"
  type        = string
}

variable "root_volume_size" {
  description = "Root EBS volume size in GB"
  type        = number
  default     = 30
}
