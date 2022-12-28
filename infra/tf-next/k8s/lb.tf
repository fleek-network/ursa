resource "digitalocean_loadbalancer" "main" {
  name        = "ursa-${var.region}-lb"
  region      = var.region
  size        = "lb-small"
  droplet_tag = var.do_tag_ursa_node

  forwarding_rule {
    entry_port     = 80
    entry_protocol = "http"

    target_port     = 80
    target_protocol = "http"
  }
  
  lifecycle {
      ignore_changes = [
        forwarding_rule,
    ]
  }
}
