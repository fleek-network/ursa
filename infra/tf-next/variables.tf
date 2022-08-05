################
#   Project    #
################

variable "project_name" {
  type        = string
  default     = "Ursa Testnet"
  description = "Name of the project in DigitalOcean"
}

variable "project_description" {
  type        = string
  default     = "Ursa testnet resources"
  description = "Description of the project in DigitalOcean"
}

variable "project_purpose" {
  type        = string
  default     = "Nebula"
  description = "Purpose of the project in DigitalOcean"
}

variable "project_stage" {
  type        = string
  default     = "Development"
  description = "Stage of the project in DigitalOcean"
}

################
#   Droplet    #
################

variable "k8s_droplet_size" {
  type        = string
  default     = "s-4vcpu-8gb"
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