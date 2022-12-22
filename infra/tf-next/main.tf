terraform {
  cloud {
    organization = "fleek-network"

    workspaces {
      name = "ursa-testnet"
    }
  }
}

provider "digitalocean" {
  token = var.do_token
}