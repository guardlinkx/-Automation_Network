# Lightweight mesh relay node for decentralized VPN routing
# Uses t3.small for cost efficiency across many regions

variable "region" {
  description = "AWS region for this relay node"
  type        = string
}

variable "instance_type" {
  description = "EC2 instance type"
  type        = string
  default     = "t3.small"
}

variable "key_name" {
  description = "SSH key pair name"
  type        = string
}

variable "node_secret" {
  description = "Shared secret for node API authentication"
  type        = string
  sensitive   = true
}

variable "central_api_ip" {
  description = "IP of the central CalifaxVPN API server"
  type        = string
  default     = "34.233.74.39"
}

variable "mesh_port" {
  description = "Port for mesh P2P communication"
  type        = number
  default     = 4001
}

provider "aws" {
  region = var.region
}

data "aws_ami" "ubuntu" {
  most_recent = true
  owners      = ["099720109477"]
  filter {
    name   = "name"
    values = ["ubuntu/images/hvm-ssd/ubuntu-jammy-22.04-amd64-server-*"]
  }
}

resource "aws_security_group" "mesh_relay" {
  name_prefix = "califax-mesh-relay-"
  description = "Security group for CalifaxVPN mesh relay node"

  # WireGuard
  ingress {
    from_port   = 51820
    to_port     = 51820
    protocol    = "udp"
    cidr_blocks = ["0.0.0.0/0"]
    description = "WireGuard VPN"
  }

  # Mesh P2P (libp2p)
  ingress {
    from_port   = var.mesh_port
    to_port     = var.mesh_port
    protocol    = "tcp"
    cidr_blocks = ["0.0.0.0/0"]
    description = "Mesh P2P communication"
  }

  ingress {
    from_port   = var.mesh_port
    to_port     = var.mesh_port
    protocol    = "udp"
    cidr_blocks = ["0.0.0.0/0"]
    description = "Mesh P2P communication (UDP)"
  }

  # Node API (restricted to central API)
  ingress {
    from_port   = 8443
    to_port     = 8443
    protocol    = "tcp"
    cidr_blocks = ["${var.central_api_ip}/32"]
    description = "Node API from central server"
  }

  # SSH
  ingress {
    from_port   = 22
    to_port     = 22
    protocol    = "tcp"
    cidr_blocks = ["${var.central_api_ip}/32"]
    description = "SSH from central server"
  }

  egress {
    from_port   = 0
    to_port     = 0
    protocol    = "-1"
    cidr_blocks = ["0.0.0.0/0"]
    description = "Allow all outbound"
  }

  tags = {
    Name = "califax-mesh-relay-${var.region}"
    Role = "mesh-relay"
  }
}

resource "aws_instance" "mesh_relay" {
  ami                    = data.aws_ami.ubuntu.id
  instance_type          = var.instance_type
  key_name               = var.key_name
  vpc_security_group_ids = [aws_security_group.mesh_relay.id]

  root_block_device {
    volume_type = "gp3"
    volume_size = 20
    encrypted   = true
  }

  user_data = templatefile("${path.module}/user_data.sh", {
    node_secret = var.node_secret
    region      = var.region
    mesh_port   = var.mesh_port
  })

  metadata_options {
    http_endpoint = "enabled"
    http_tokens   = "required"
  }

  tags = {
    Name        = "califax-mesh-relay-${var.region}"
    Role        = "mesh-relay"
    ManagedBy   = "terraform"
  }
}

output "relay_ip" {
  value = aws_instance.mesh_relay.public_ip
}

output "relay_id" {
  value = aws_instance.mesh_relay.id
}
