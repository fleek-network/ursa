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
  region = "us-east-1"
}

# Override with TF_VAR_client_count
variable "client_count" {
  type        = number
  default     = 5
  description = "Number of clients to deploy"
}

data "aws_ami" "ufdp_benchmark_image" {
  most_recent = true
  filter {
    name   = "name"
    values = ["debian-11-amd64-*"]
  }
}

resource "aws_key_pair" "ufdp_benchmark_key" {
  key_name   = "ufdp_benchmark_key"
  public_key = file("benchmark_key.pub")
}

resource "aws_instance" "ufdp_benchmark_client" {
  count         = var.client_count
  instance_type = "m5n.xlarge"
  ami           = data.aws_ami.ufdp_benchmark_image.id
  key_name      = aws_key_pair.ufdp_benchmark_key.key_name

  tags = {
    Name = "benchmark-client-${count.index}"
  }

  connection {
    type        = "ssh"
    user        = "admin"
    private_key = file("benchmark_key")
    host        = self.public_ip
  }

  # upload our client binary
  provisioner "file" {
    source      = "../../target/release/ufdp-bench-client"
    destination = "ufdp-bench-client"
  }

  # make it executable
  provisioner "remote-exec" {
    inline = [
      "chmod +x ufdp-bench-client",
      "sudo mv ufdp-bench-client /usr/bin"
    ]
  }
}

output "ip_addrs" {
  value = aws_instance.ufdp_benchmark_client[*].public_ip
}
