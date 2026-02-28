variable "aws_region" {
  description = "Primary AWS region"
  type        = string
  default     = "us-east-1"
}

variable "instance_type" {
  description = "EC2 instance type for VPN nodes"
  type        = string
  default     = "t3.medium"
}

variable "key_name" {
  description = "SSH key pair name"
  type        = string
  default     = "califaxvpn-key"
}

variable "allowed_ssh_cidr" {
  description = "CIDR block allowed SSH access"
  type        = string
  default     = "0.0.0.0/0"
}

variable "central_api_ip" {
  description = "IP of the central GuardLinkX API server"
  type        = string
  default     = "34.233.74.39"
}

variable "node_api_secret" {
  description = "Shared secret for node API authentication"
  type        = string
  sensitive   = true
}
