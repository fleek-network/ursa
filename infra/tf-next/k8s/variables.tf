variable "ghcr_registry_username" {
}
variable "ghcr_registry_password" {
}

variable "do_project_id" {
}
variable "do_region" {
}

variable "registry_server" {
  default = "ghcr.io"
}

variable "k8s_ursa_docker_image" {
  default = "nginx"
}

variable "k8s_droplet_size" {
  type        = string
  default     = "s-2vcpu-4gb"
  description = "Default k8s droplet size identifier"
}

variable "k8s_min_node_count" {
  default     = 3
  type        = number
  description = "How many testnet nodes to deploy"
}

variable "k8s_max_node_count" {
  default     = 6
  type        = number
  description = "How many testnet nodes to deploy"
}