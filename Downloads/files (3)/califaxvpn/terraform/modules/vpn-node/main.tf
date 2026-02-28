variable "region" {
  type = string
}

variable "instance_type" {
  type    = string
  default = "t3.medium"
}

variable "key_name" {
  type = string
}

variable "allowed_ssh_cidr" {
  type    = string
  default = "0.0.0.0/0"
}

variable "central_api_ip" {
  type = string
}

variable "node_api_secret" {
  type      = string
  sensitive = true
}

variable "vpn_region_label" {
  type = string
}

# Latest Ubuntu 22.04 LTS AMI
data "aws_ami" "ubuntu" {
  most_recent = true
  owners      = ["099720109477"] # Canonical

  filter {
    name   = "name"
    values = ["ubuntu/images/hvm-ssd/ubuntu-jammy-22.04-amd64-server-*"]
  }

  filter {
    name   = "virtualization-type"
    values = ["hvm"]
  }
}

resource "aws_security_group" "vpn_node" {
  name_prefix = "califaxvpn-node-${var.vpn_region_label}-"
  description = "Califax VPN node security group"

  # WireGuard UDP
  ingress {
    from_port   = 51820
    to_port     = 51820
    protocol    = "udp"
    cidr_blocks = ["0.0.0.0/0"]
    description = "WireGuard tunnel"
  }

  # Node API — restricted to central API server
  ingress {
    from_port   = 8443
    to_port     = 8443
    protocol    = "tcp"
    cidr_blocks = ["${var.central_api_ip}/32"]
    description = "Node API (central API only)"
  }

  # SSH
  ingress {
    from_port   = 22
    to_port     = 22
    protocol    = "tcp"
    cidr_blocks = [var.allowed_ssh_cidr]
    description = "SSH access"
  }

  egress {
    from_port   = 0
    to_port     = 0
    protocol    = "-1"
    cidr_blocks = ["0.0.0.0/0"]
  }

  tags = {
    Name    = "califaxvpn-node-${var.vpn_region_label}"
    Project = "CalifaxVPN"
  }
}

resource "aws_instance" "vpn_node" {
  ami                    = data.aws_ami.ubuntu.id
  instance_type          = var.instance_type
  key_name               = var.key_name
  vpc_security_group_ids = [aws_security_group.vpn_node.id]

  user_data = templatefile("${path.module}/user_data.sh", {
    node_api_secret  = var.node_api_secret
    vpn_region_label = var.vpn_region_label
  })

  root_block_device {
    volume_size = 20
    volume_type = "gp3"
    encrypted   = true
  }

  tags = {
    Name    = "califaxvpn-node-${var.vpn_region_label}"
    Project = "CalifaxVPN"
    Region  = var.vpn_region_label
  }
}

resource "aws_eip" "vpn_node" {
  instance = aws_instance.vpn_node.id
  domain   = "vpc"

  tags = {
    Name    = "califaxvpn-eip-${var.vpn_region_label}"
    Project = "CalifaxVPN"
  }
}

output "public_ip" {
  value = aws_eip.vpn_node.public_ip
}

output "instance_id" {
  value = aws_instance.vpn_node.id
}
