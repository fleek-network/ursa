resource "digitalocean_firewall" "ursa_peering" {
  name = "ursa-peering"

  tags = ["ursa-main"]

  inbound_rule {
    protocol         = "tcp"
    port_range       = "6009"
    source_addresses = ["0.0.0.0/0", "::/0"]
  }

  inbound_rule {
    protocol         = "udp"
    port_range       = "6009"
    source_addresses = ["0.0.0.0/0", "::/0"]
  }

  outbound_rule {
    protocol              = "tcp"
    port_range            = "6009"
    destination_addresses = ["0.0.0.0/0", "::/0"]
  }

  outbound_rule {
    protocol              = "udp"
    port_range            = "6009"
    destination_addresses = ["0.0.0.0/0", "::/0"]
  }
}