terraform {
  required_providers {
    fly = {
      source = "fly-apps/fly"
      version = "0.0.16"
    }
  }
}

resource "fly_app" "ursa_gateway" {
  name = "ursa-gateway"
  org  = "fleek-network"
}

resource "fly_ip" "ursa_gateway_ip" {
  app        = "ursa-gateway"
  type       = "v4"
  depends_on = [fly_app.ursa_gateway]
}

resource "fly_ip" "ursa_gateway_ipv6" {
  app        = "ursa-gateway"
  type       = "v6"
  depends_on = [fly_app.ursa_gateway]
}