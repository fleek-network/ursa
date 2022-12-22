resource "digitalocean_firewall" "ursa_peering" {
  name = "ursa-peering"

  tags = ["ursa-main", "ursa-bootstrap"]

  # Allow SSH
  inbound_rule {
    protocol         = "tcp"
    port_range       = "22"
    source_addresses = ["0.0.0.0/0"]
  }

  # Allow Tcp p2p port for ursa node
  inbound_rule {
    protocol         = "tcp"
    port_range       = "6009"
    source_addresses = ["0.0.0.0/0", "::/0"]
  }

  # Allow Quic p2p port for ursa node
  inbound_rule {
    protocol         = "udp"
    port_range       = "6009"
    source_addresses = ["0.0.0.0/0", "::/0"]
  }

  # Allow HTTP/HTTPS ingress from load balancer
  # todo(botch): custom load balancer port
  # inbound_rule {
  #   protocol         = "tcp"
  #   port_range       = "443"
  #   source_addresses = [digitalocean_loadbalancer.main.id]
  # }

  # Allow HTTP/HTTPS ingress
  inbound_rule {
    protocol         = "tcp"
    port_range       = "80"
    source_addresses = ["0.0.0.0/0", "::/0"]
  }

  inbound_rule {
    protocol         = "tcp"
    port_range       = "443"
    source_addresses = ["0.0.0.0/0", "::/0"]
  }

  # Allow Tcp p2p port for ursa node
  outbound_rule {
    protocol              = "tcp"
    port_range            = "6009"
    destination_addresses = ["0.0.0.0/0", "::/0"]
  }

  # Allow Quic p2p port for ursa node
  outbound_rule {
    protocol              = "udp"
    port_range            = "6009"
    destination_addresses = ["0.0.0.0/0", "::/0"]
  }

  # Allow all outbound traffic
  # todo(botch): can we restrict this more
  outbound_rule {
    protocol              = "tcp"
    port_range            = "1-65535"
    destination_addresses = ["0.0.0.0/0", "::/0"]
  }

  outbound_rule {
    protocol              = "udp"
    port_range            = "1-65535"
    destination_addresses = ["0.0.0.0/0", "::/0"]
  }

  outbound_rule {
    protocol              = "icmp"
    port_range            = "1-65535"
    destination_addresses = ["0.0.0.0/0", "::/0"]
  }
}