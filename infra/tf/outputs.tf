output "testnet_nodes_ip_address" {
  value       = digitalocean_droplet.testnet-node.*.ipv4_address
  description = "Testnet node IP addresses"
}

output "bootstrap_nodes_ip_address" {
  value       = digitalocean_droplet.bootstrap-node.*.ipv4_address
  description = "Bootstrap node IP addresses."
}
