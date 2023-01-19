terraform {
  required_providers {
    fly = {
      source = "fly-apps/fly"
      version = "0.0.20"
    }
  }
}

provider "fly" {
  useinternaltunnel    = true
  internaltunnelorg    = "fleek-network"
  internaltunnelregion = "yyz"
}

resource "fly_app" "ursa_gateway" {
  name = "ursa-gateway"
  org  = "fleek-network"
}

# resource "fly_volume" "ursa_gateway_volume" {
#   app    = "ursa-gateway"
#   name   = "ursa_gateway_volume"
#   size   = 15
#   region = "yyz"

#   depends_on = [fly_app.ursa_gateway]
# }

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