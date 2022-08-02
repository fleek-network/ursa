output "testnet_nodes_ip_address" {
  value       = digitalocean_droplet.testnet-node.*.ipv4_address
  description = "The public IP address of your Droplet application."
}

output "bootstrap_nodes_ip_address" {
  value       = digitalocean_droplet.bootstrap-node.*.ipv4_address
  description = "The public IP address of your Droplet application."
}
