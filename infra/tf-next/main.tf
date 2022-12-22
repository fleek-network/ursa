terraform {
  cloud {
    organization = "fleek-network"

    workspaces {
      name = "ursa-testnet"
    }
  }
}