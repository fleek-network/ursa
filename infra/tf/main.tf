resource "digitalocean_domain" "default" {
  name = var.ursa_domain
}

resource "digitalocean_record" "static" {
  count  = var.worker_count
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

resource "digitalocean_project_resources" "domain" {
  project   = digitalocean_project.ursa-dev.id
  resources = [digitalocean_domain.default.urn]
}

resource "digitalocean_droplet" "testnet-node" {
  count      = var.worker_count
  image      = var.droplet_image
  name       = "testnet-node-${count.index}"
  region     = var.droplet_region
  size       = var.droplet_size
  backups    = false
  monitoring = true
  # user_data  = file("bootstrap_node.yml")
  user_data = file(format("%s/bootstrap_node.yml", path.module))
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

resource "digitalocean_firewall" "testnet-node" {
  count = var.worker_count
  name  = "only-allow-ssh-http-and-https"

  droplet_ids = (digitalocean_droplet.testnet-node[*].id)

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