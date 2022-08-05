terraform {
  cloud {
    organization = "fleek"

    workspaces {
      name = "ursa-testnet"
    }
  }
}

resource "digitalocean_project" "ursa" {
  name        = var.project_name
  description = var.project_description
  purpose     = var.project_purpose
  environment = var.project_stage
}