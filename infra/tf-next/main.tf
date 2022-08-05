terraform {
  cloud {
    organization = "fleek"

    workspaces {
      name = "ursa-testnet"
    }
  }
}