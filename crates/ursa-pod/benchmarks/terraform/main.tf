# AWS Setup, requires `AWS_ACCESS_KEY_ID` and `AWS_SECRET_ACCESS_KEY`
terraform {
  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = "~> 4.16"
    }
  }
  required_version = ">= 1.2.0"
}

provider "aws" {
  region  = "eu-north-1"
}

# Base Image
data "aws_ami" "bullseye" {
  most_recent = true

  filter {
    name   = "name"
    values = ["debian-11-amd64-*"]
  }
}

# Instances
resource "aws_instance" "benchmark-client" {
  #count         = 1
  instance_type = "t2.micro"
  ami           = data.aws_ami.bullseye.id
  tags = {
    #Name = "benchmark-client-${count.index}"
  }
}

#output "ip_addrs" {
#  value = aws_instance.benchmark-client[*].public_ip
#}
