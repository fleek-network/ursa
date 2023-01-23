################
#   Project    #
################

variable "do_project_id" {
  type        = string
  description = "The digital ocean project id"
}

variable "default_droplet_size" {
  type        = string
  default     = "s-2vcpu-4gb"
  description = "Default k8s droplet size identifier"
}

variable "k8s_domains" {
  type        = list(string)
  description = "Top level domains to create records and pods for"
}

variable "lets_encrypt_env" {
  type        = string
  description = "Lets Encrypt `staging` or `prod`"
}

variable "do_tag_ursa_node" {
  type        = string
  description = "Ursa node tag."
}

variable "do_tag_ursa_bs_node" {
  type        = string
  description = "Ursa bootstrap tag."
}

################
#   Cluster    #
################

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

variable "cluster_name" {
  type        = string
  description = "The name of the kubernetes cluster to create"
}

variable "cluster_version" {
  type        = string
  description = "The version of the kubernetes cluster to create"
}

variable "region" {
  type        = string
  default     = "ams3"
  description = "The digital ocean region slug for where to create resources"
}

###########
# Images  #
###########
variable "k8s_ursa_docker_image" {
  default = "ghcr.io/fleek-network/ursa:latest"
}

###########
# ghcr.io #
###########

variable "registry_server" {
  type    = string
  default = "ghcr.io"
}

variable "letsencrypt_email" {
  type        = string
  default     = "major@ursa.earth"
  description = "Let's Encrypt admin email"
}