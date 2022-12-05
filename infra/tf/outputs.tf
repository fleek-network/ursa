output "testnet_nodes_ip_address" {
  value       = digitalocean_droplet.testnet-node.*.ipv4_address
  description = "Testnet node IP addresses"
}

output "bootstrap_nodes_ip_address" {
  value       = digitalocean_droplet.bootstrap-node.*.ipv4_address
  description = "Bootstrap node IP addresses."
}

output "testnet_prometheus_ip_address" {
  value       = digitalocean_droplet.testnet-prometheus.ipv4_address
  description = "Prometheus IP address"
}

output "testnet_grafana_ip_address" {
  value       = digitalocean_droplet.testnet-grafana.ipv4_address
  description = "Grana IP address."
}

output "testnet_tracker_ip_address" {
  value       = digitalocean_droplet.testnet-http-tracker.ipv4_address
  description = "Tracker IP address."
}
