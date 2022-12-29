terraform {
  cloud {
    organization = "fleek-network"

    workspaces {
      name = "ursa-testnet"
    }
  }
}

# For local testing with tfvars
# provider "digitalocean" {
#   token = var.do_token
# }