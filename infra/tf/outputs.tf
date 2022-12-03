output "testnet_nodes_ip_address" {
  value       = digitalocean_droplet.testnet-node.*.ipv4_address
  description = "The public IP address of your Droplet application."
}

output "bootstrap_nodes_ip_address" {
  value       = digitalocean_droplet.bootstrap-node.*.ipv4_address
  description = "The public IP address of your Droplet application."
}

output "testnet_prometheus_ip_address" {
  value       = digitalocean_droplet.testnet-prometheus.ipv4_address
  description = "The public IP address of your Droplet application."
}

output "testnet_grafana_ip_address" {
  value       = digitalocean_droplet.testnet-grafana.ipv4_address
  description = "The public IP address of your Droplet application."
}

output "testnet_tracker_ip_address" {
  value       = digitalocean_droplet.testnet-http-tracker.ipv4_address
  description = "The public IP address of your Droplet application."
}
