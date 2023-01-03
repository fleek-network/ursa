resource "digitalocean_firewall" "ursa_peering" {
  name = "ursa-peering"
  tags = [var.do_tag_ursa_node, var.do_tag_ursa_bs_node]

  # SSH
  inbound_rule {
    protocol         = "tcp"
    port_range       = "22"
    source_addresses = ["0.0.0.0/0"]
  }

  # Tcp p2p port for ursa node
  inbound_rule {
    protocol         = "tcp"
    port_range       = "6009"
    source_addresses = ["0.0.0.0/0", "::/0"]
  }

  # Inbound Quic p2p port for ursa node
  inbound_rule {
    protocol         = "udp"
    port_range       = "6009"
    source_addresses = ["0.0.0.0/0", "::/0"]
  }

  # Inbound HTTP/HTTPS ingress from load balancer
  inbound_rule {
    protocol                  = "tcp"
    port_range                = "4069"
    source_load_balancer_uids = [digitalocean_loadbalancer.main.id]
  }

  # Inbound access from cluster nodes
  inbound_rule {
    protocol    = "tcp"
    port_range  = "1-65535"
    source_tags = [var.do_tag_ursa_node, var.do_tag_ursa_bs_node]
  }

  inbound_rule {
    protocol    = "udp"
    port_range  = "1-65535"
    source_tags = [var.do_tag_ursa_node, var.do_tag_ursa_bs_node]
  }

  inbound_rule {
    protocol    = "icmp"
    source_tags = [var.do_tag_ursa_node, var.do_tag_ursa_bs_node]
  }

  # Outbound Tcp p2p port for ursa node
  outbound_rule {
    protocol              = "tcp"
    port_range            = "6009"
    destination_addresses = ["0.0.0.0/0", "::/0"]
  }

  # Outbound Quic p2p port for ursa node
  outbound_rule {
    protocol              = "udp"
    port_range            = "6009"
    destination_addresses = ["0.0.0.0/0", "::/0"]
  }

  # HTTP
  outbound_rule {
    protocol              = "tcp"
    port_range            = "80"
    destination_addresses = ["0.0.0.0/0", "::/0"]
  }

  # HTTPS
  outbound_rule {
    protocol              = "tcp"
    port_range            = "443"
    destination_addresses = ["0.0.0.0/0", "::/0"]
  }

  # Allow all outbound traffic - todo(botch): remove
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

  # Ping
  outbound_rule {
    protocol              = "icmp"
    destination_addresses = ["0.0.0.0/0", "::/0"]
  }
}