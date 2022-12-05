resource "digitalocean_domain" "default" {
  name = var.ursa_domain
}

resource "digitalocean_record" "static" {
  count  = var.node_count
  domain = digitalocean_domain.default.name
  type   = "A"
  name   = digitalocean_droplet.testnet-node[count.index].name
  value  = digitalocean_droplet.testnet-node[count.index].ipv4_address
}

resource "digitalocean_project" "ursa-dev" {
  name        = var.project_name
  description = var.project_description
  purpose     = var.project_purpose
  environment = var.project_stage
}

resource "digitalocean_project_resources" "node_droplets" {
  project   = digitalocean_project.ursa-dev.id
  resources = digitalocean_droplet.testnet-node[*].urn
}

resource "digitalocean_project_resources" "bootstrap_droplets" {
  project   = digitalocean_project.ursa-dev.id
  resources = digitalocean_droplet.bootstrap-node[*].urn
}

resource "digitalocean_project_resources" "domain" {
  project   = digitalocean_project.ursa-dev.id
  resources = [digitalocean_domain.default.urn]
}

resource "digitalocean_droplet" "testnet-node" {
  count      = var.node_count
  image      = var.droplet_image
  name       = "testnet-node-${count.index}"
  region     = var.droplet_region
  size       = var.droplet_size
  backups    = false
  monitoring = true
  user_data = file(format("%s/configs/bootstrap_node.yml", path.module))
  ssh_keys = [
    data.digitalocean_ssh_key.ursa-dev.id
  ]

  # Use the below block for new ssh keys
  # connection {
  #   host        = self.ipv4_address
  #   user        = "root"
  #   type        = "ssh"
  #   private_key = file(var.pvt_key)
  #   timeout     = "2m"
  # }

  # provisioner "remote-exec" {
  #   inline = [
  #     "export PATH=$PATH:/usr/bin",
  #     "sudo apt update",
  #   ]
  # }
}

resource "digitalocean_droplet" "bootstrap-node" {
  count      = var.bootstrap_count
  image      = var.droplet_image
  name       = "bootstrap-node-${count.index}"
  region     = var.droplet_region
  size       = var.droplet_size
  backups    = false
  monitoring = true
  user_data  = file("${path.module}/configs/bootstrap_node.yml")
  ssh_keys = [
    data.digitalocean_ssh_key.ursa-dev.id
  ]
}

resource "digitalocean_firewall" "ursa-network" {
  count = 1
  name  = "ursa-network-only-allow-ssh-http-and-https"
  droplet_ids = concat(digitalocean_droplet.testnet-node[*].id, digitalocean_droplet.bootstrap-node[*].id)

  inbound_rule {
    protocol         = "tcp"
    port_range       = "22"
    source_addresses = ["0.0.0.0/0", "::/0"]
  }
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
  inbound_rule {
    port_range = "4070"
    protocol   = "tcp"
    source_addresses = ["0.0.0.0/0", "::/0"]
  }
  inbound_rule {
    port_range = "6009"
    protocol   = "tcp"
    source_addresses = ["0.0.0.0/0", "::/0"]
  }
  inbound_rule {
    protocol         = "icmp"
    source_addresses = ["0.0.0.0/0", "::/0"]
  }

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
    destination_addresses = ["0.0.0.0/0", "::/0"]
  }
}

# prometheus.ursa.earth
resource "digitalocean_record" "prometheus-domain" {
  domain = digitalocean_domain.default.name
  type   = "A"
  name   = digitalocean_droplet.testnet-prometheus.name
  value  = digitalocean_droplet.testnet-prometheus.ipv4_address
}

resource "digitalocean_droplet" "testnet-prometheus" {
  image      = var.droplet_image
  name       = "prometheus"
  region     = var.droplet_region
  size       = var.droplet_size
  backups    = false
  monitoring = true
  user_data  = templatefile("${path.module}/configs/prometheus.conf", {
    domain = var.ursa_domain,
    email = var.letsencrypt_email
  })
  ssh_keys = [
    data.digitalocean_ssh_key.ursa-dev.id
  ]
}

# grafana.ursa.earth
resource "digitalocean_record" "grafana-domain" {
  domain = digitalocean_domain.default.name
  type   = "A"
  name   = digitalocean_droplet.testnet-grafana.name
  value  = digitalocean_droplet.testnet-grafana.ipv4_address
}

resource "digitalocean_droplet" "testnet-grafana" {
  image      = var.droplet_image
  name       = "grafana"
  region     = var.droplet_region
  size       = var.droplet_size
  backups    = false
  monitoring = true
  user_data  = templatefile("${path.module}/configs/grafana/cloud.conf", {
    domain = var.ursa_domain,
    email = var.letsencrypt_email
    ursa_dashboard = file("${path.module}/configs/grafana/ursa.json")
  })
  ssh_keys = [
    data.digitalocean_ssh_key.ursa-dev.id
  ]
}

# tracker.ursa.earth
resource "digitalocean_record" "http-tracker-domain" {
  domain = digitalocean_domain.default.name
  type   = "A"
  name   = digitalocean_droplet.testnet-http-tracker.name
  value  = digitalocean_droplet.testnet-http-tracker.ipv4_address
}

resource "digitalocean_droplet" "testnet-http-tracker" {
  image      = "ubuntu-22-10-x64"
  name       = "tracker"
  region     = var.droplet_region
  size       = var.droplet_size
  backups    = false
  monitoring = true
  user_data  = templatefile("${path.module}/configs/http_tracker.conf", {
    ipinfo_token = var.ipinfo_token
  })
  ssh_keys = [
    data.digitalocean_ssh_key.ursa-dev.id
  ]
}

# Grafana, Prometheus, and HTTP Tracker share rules
resource "digitalocean_firewall" "metrics-firewall" {
  name  = "metrics-firewall"
  droplet_ids = [
    digitalocean_droplet.testnet-http-tracker.id,
    digitalocean_droplet.testnet-grafana.id,
    digitalocean_droplet.testnet-prometheus.id
  ]
  # ssh, http, https
  inbound_rule {
    protocol         = "tcp"
    port_range       = "22"
    source_addresses = ["0.0.0.0/0", "::/0"]
  }
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
  inbound_rule {
    protocol         = "tcp"
    port_range       = "3000"
    source_addresses = ["0.0.0.0/0", "::/0"]
  }
  inbound_rule {
    protocol         = "tcp"
    port_range       = "4000"
    source_addresses = ["0.0.0.0/0", "::/0"]
  }
  inbound_rule {
    protocol         = "tcp"
    port_range       = "9090"
    source_addresses = ["0.0.0.0/0", "::/0"]
  }
  inbound_rule {
    protocol         = "icmp"
    source_addresses = ["0.0.0.0/0", "::/0"]
  }
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
    destination_addresses = ["0.0.0.0/0", "::/0"]
  }
}
