# AWS CloudHSM cluster for hardware key management

variable "region" {
  type    = string
  default = "us-east-1"
}

variable "vpc_id" {
  type        = string
  description = "VPC ID for CloudHSM cluster"
}

variable "subnet_ids" {
  type        = list(string)
  description = "Subnet IDs for CloudHSM (must be in different AZs)"
}

provider "aws" {
  region = var.region
}

resource "aws_cloudhsm_v2_cluster" "califax_hsm" {
  hsm_type   = "hsm1.medium"
  subnet_ids = var.subnet_ids

  tags = {
    Name      = "califax-hsm-cluster"
    ManagedBy = "terraform"
  }
}

resource "aws_cloudhsm_v2_hsm" "califax_hsm_instance" {
  cluster_id = aws_cloudhsm_v2_cluster.califax_hsm.cluster_id
  subnet_id  = var.subnet_ids[0]
}

output "cluster_id" {
  value = aws_cloudhsm_v2_cluster.califax_hsm.cluster_id
}

output "cluster_state" {
  value = aws_cloudhsm_v2_cluster.califax_hsm.cluster_state
}

output "hsm_id" {
  value = aws_cloudhsm_v2_hsm.califax_hsm_instance.hsm_id
}
