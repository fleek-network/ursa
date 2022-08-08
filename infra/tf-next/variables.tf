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

###########
# ghcr.io #
###########
variable "ghcr_registry_username" {
  default = "user"
}
variable "ghcr_registry_password" {
  default = "pass"
}

###########
# Images  #
###########
variable "k8s_ursa_docker_image" {
  default = "nginx"
}