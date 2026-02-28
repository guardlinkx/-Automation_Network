terraform {
  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = "~> 5.0"
    }
  }
  required_version = ">= 1.5.0"
}

provider "aws" {
  region = var.aws_region
}

# Phase 1: US East only
module "vpn_us_east" {
  source = "./modules/vpn-node"

  region           = "us-east-1"
  instance_type    = var.instance_type
  key_name         = var.key_name
  allowed_ssh_cidr = var.allowed_ssh_cidr
  central_api_ip   = var.central_api_ip
  node_api_secret  = var.node_api_secret
  vpn_region_label = "us-east-1"
}

# Phase 2: Uncomment to expand
# provider "aws" {
#   alias  = "eu_central"
#   region = "eu-central-1"
# }
#
# module "vpn_eu_central" {
#   source    = "./modules/vpn-node"
#   providers = { aws = aws.eu_central }
#
#   region           = "eu-central-1"
#   instance_type    = var.instance_type
#   key_name         = var.key_name
#   allowed_ssh_cidr = var.allowed_ssh_cidr
#   central_api_ip   = var.central_api_ip
#   node_api_secret  = var.node_api_secret
#   vpn_region_label = "eu-central-1"
# }
#
# provider "aws" {
#   alias  = "ap_northeast"
#   region = "ap-northeast-1"
# }
#
# module "vpn_ap_northeast" {
#   source    = "./modules/vpn-node"
#   providers = { aws = aws.ap_northeast }
#
#   region           = "ap-northeast-1"
#   instance_type    = var.instance_type
#   key_name         = var.key_name
#   allowed_ssh_cidr = var.allowed_ssh_cidr
#   central_api_ip   = var.central_api_ip
#   node_api_secret  = var.node_api_secret
#   vpn_region_label = "ap-northeast-1"
# }
#
# provider "aws" {
#   alias  = "eu_west"
#   region = "eu-west-2"
# }
#
# module "vpn_eu_west" {
#   source    = "./modules/vpn-node"
#   providers = { aws = aws.eu_west }
#
#   region           = "eu-west-2"
#   instance_type    = var.instance_type
#   key_name         = var.key_name
#   allowed_ssh_cidr = var.allowed_ssh_cidr
#   central_api_ip   = var.central_api_ip
#   node_api_secret  = var.node_api_secret
#   vpn_region_label = "eu-west-2"
# }
#
# provider "aws" {
#   alias  = "ca_central"
#   region = "ca-central-1"
# }
#
# module "vpn_ca_central" {
#   source    = "./modules/vpn-node"
#   providers = { aws = aws.ca_central }
#
#   region           = "ca-central-1"
#   instance_type    = var.instance_type
#   key_name         = var.key_name
#   allowed_ssh_cidr = var.allowed_ssh_cidr
#   central_api_ip   = var.central_api_ip
#   node_api_secret  = var.node_api_secret
#   vpn_region_label = "ca-central-1"
# }
