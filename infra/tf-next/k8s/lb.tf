resource "digitalocean_tag" "ursa_node" {
  name = "ursa-node-${var.region}"
}

resource "digitalocean_loadbalancer" "main" {
  name        = "ursa-${var.region}-lb"
  region      = var.region
  size        = "lb-small"
  droplet_tag = digitalocean_tag.ursa_node.id

  forwarding_rule {
    entry_port     = 80
    entry_protocol = "http"

    target_port     = 4069
    target_protocol = "http"
  }

  healthcheck {
    port                     = 4069
    protocol                 = "http"
    path                     = "/"
    check_interval_seconds   = 5
    response_timeout_seconds = 3
    unhealthy_threshold      = 2
    healthy_threshold        = 2
  }
}
