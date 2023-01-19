resource "fly_machine" "ursa-gateway-machine" {
  for_each = toset(var.regions)
  app    = "ursa-gateway"
  region = each.value
  name   = "ursa-gateway-${each.value}"
  image  = "ghcr.io/fleek-network/ursa-gateway:latest"
  services = [
    {
      ports = [
        {
          port     = 443
          handlers = ["tls", "http"]
        },
        {
          port     = 80
          handlers = ["http"]
        }
      ]
      "protocol" : "tcp",
      "internal_port" : 80
    },
  ]
  cpus = 4
  memorymb = 8192
  depends_on = [fly_app.ursa_gateway]
}