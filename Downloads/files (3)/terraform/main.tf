##############################################################################
# GuardLinkX Storefront — AWS Infrastructure
##############################################################################

terraform {
  required_version = ">= 1.0"
  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = "~> 5.0"
    }
  }
}

provider "aws" {
  region = var.aws_region
}

# ---------------------------------------------------------------------------
# Data sources
# ---------------------------------------------------------------------------

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

# Default VPC
data "aws_vpc" "default" {
  default = true
}

# ---------------------------------------------------------------------------
# SSH Key Pair — using existing key pair registered in AWS
# ---------------------------------------------------------------------------

# ---------------------------------------------------------------------------
# Security Group
# ---------------------------------------------------------------------------

resource "aws_security_group" "guardlinkx" {
  name        = "guardlinkx-sg"
  description = "GuardLinkX storefront - SSH, HTTP, HTTPS"
  vpc_id      = data.aws_vpc.default.id

  # SSH
  ingress {
    description = "SSH"
    from_port   = 22
    to_port     = 22
    protocol    = "tcp"
    cidr_blocks = [var.allowed_ssh_cidr]
  }

  # HTTP
  ingress {
    description = "HTTP"
    from_port   = 80
    to_port     = 80
    protocol    = "tcp"
    cidr_blocks = ["0.0.0.0/0"]
  }

  # HTTPS
  ingress {
    description = "HTTPS"
    from_port   = 443
    to_port     = 443
    protocol    = "tcp"
    cidr_blocks = ["0.0.0.0/0"]
  }

  # All outbound
  egress {
    from_port   = 0
    to_port     = 0
    protocol    = "-1"
    cidr_blocks = ["0.0.0.0/0"]
  }

  tags = {
    Name = "guardlinkx-sg"
  }
}

# ---------------------------------------------------------------------------
# EC2 Instance
# ---------------------------------------------------------------------------

resource "aws_instance" "guardlinkx" {
  ami                    = data.aws_ami.ubuntu.id
  instance_type          = var.instance_type
  key_name               = var.key_name
  vpc_security_group_ids = [aws_security_group.guardlinkx.id]

  root_block_device {
    volume_size = var.root_volume_size
    volume_type = "gp3"
    encrypted   = true
  }

  user_data = templatefile("${path.module}/user_data.sh", {
    domain_name            = var.domain_name
    db_password            = var.db_password
    flask_secret_key       = var.flask_secret_key
    stripe_secret_key      = var.stripe_secret_key
    stripe_publishable_key = var.stripe_publishable_key
    stripe_webhook_secret  = var.stripe_webhook_secret
    admin_email            = var.admin_email
    admin_password         = var.admin_password
  })

  tags = {
    Name = "guardlinkx-storefront"
  }
}

# ---------------------------------------------------------------------------
# Elastic IP
# ---------------------------------------------------------------------------

resource "aws_eip" "guardlinkx" {
  domain = "vpc"

  tags = {
    Name = "guardlinkx-eip"
  }
}

resource "aws_eip_association" "guardlinkx" {
  instance_id   = aws_instance.guardlinkx.id
  allocation_id = aws_eip.guardlinkx.id
}
