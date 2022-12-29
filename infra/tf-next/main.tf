terraform {
  cloud {
    organization = "fleek-network"

    workspaces {
      name = "ursa-testnet"
    }
  }
}

# For local dev with tfvars
# provider "digitalocean" {
#   token = var.do_token
# }