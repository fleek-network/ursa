terraform {
  required_providers {
    fly = {
      source = "fly-apps/fly"
      version = "0.0.20"
    }
  }
}

provider "fly" {
    fly_http_endpoint = "api.machines.dev"
}

resource "fly_app" "ursa_gateway" {
  name = "ursa-gateway"
  org  = "fleek-network"
}

# resource "fly_volume" "ursa_gateway_volume" {
#   app    = fly_app.ursa_gateway.name
#   name   = "ursa_gateway_volume"
#   size   = 15
#   region = "yyz"

#   depends_on = [fly_app.ursa_gateway]
# }

resource "fly_ip" "ursa_gateway_ip" {
  app        = fly_app.ursa_gateway.name
  type       = "v4"
  depends_on = [fly_app.ursa_gateway]
}

resource "fly_ip" "ursa_gateway_ipv6" {
  app        = fly_app.ursa_gateway.name
  type       = "v6"
  depends_on = [fly_app.ursa_gateway]
}