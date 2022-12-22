resource "digitalocean_firewall" "ursa_peering" {
  name = "ursa-peering"

  tags = ["ursa-main", "ursa-bootstrap"]

  // Tcp p2p port for ursa node
  inbound_rule {
    protocol         = "tcp"
    port_range       = "6009"
    source_addresses = ["0.0.0.0/0", "::/0"]
  }

  // Quic p2p port for ursa node
  inbound_rule {
    protocol         = "udp"
    port_range       = "6009"
    source_addresses = ["0.0.0.0/0", "::/0"]
  }

  // SSH port
  inbound_rule {
    protocol         = "tcp"
    port_range       = "22"
    source_addresses = ["0.0.0.0/0"]
  }

  // Tcp p2p port for ursa node
  outbound_rule {
    protocol              = "tcp"
    port_range            = "6009"
    destination_addresses = ["0.0.0.0/0", "::/0"]
  }

  // Quic p2p port for ursa node
  outbound_rule {
    protocol              = "udp"
    port_range            = "6009"
    destination_addresses = ["0.0.0.0/0", "::/0"]
  }

  // For calling external ip addresses
  // todo(botch): restrict this more
  outbound_rule {
    protocol              = "tcp"
    port_range            = "all"
    destination_addresses = ["0.0.0.0/0"]
  }
}