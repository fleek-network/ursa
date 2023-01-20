resource "fly_machine" "ursa_gateway_machine" {
  for_each = toset(var.regions)
  app      = fly_app.ursa_gateway.name
  region   = each.value
  name     = "ursa-gateway-${each.value}"
  image    = "fleeknetwork/ursa-gateway"
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