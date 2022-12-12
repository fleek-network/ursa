################
#   Project    #
################

variable "project_name" {
  type        = string
  default     = "Ursa Testbed"
  description = "Name of the project in DigitalOcean"
}

variable "project_description" {
  type        = string
  default     = "Ursa testbed project"
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

# Use if there is a new ssh key not on DO
# variable "pvt_key" {
#   description = "SSH key"
# }

variable "droplet_image" {
  # default     = "ubuntu-20-04-x64"
  type        = string
  default     = "docker-20-04"
  description = "Image identifier of the OS in DigitalOcean"
}

variable "droplet_region" {
  type        = string
  default     = "ams3"
  description = "Droplet region identifier where the droplet will be created"
}

variable "droplet_size" {
  type        = string
  default     = "s-2vcpu-4gb"
  description = "Droplet size identifier"
}

variable "droplet_volume" {
  type        = number
  default     = 1024
  description = "Droplet volume size in GB"
}

#############
#   Misc    #
#############

variable "letsencrypt_email" {
  type    = string
  default = "admin@ursa.earth"
}

# export variables in the format `TF_VAR_xyz` to expose them to terraform
# TF_VAR_do_token=
variable "do_token" {
  description = "DigitalOcean API token"
}

# TF_VAR_ipinfo_token
variable "ipinfo_token" {
  type       = string
  description = "ipinfo.io API Token"
}

variable "ursa_domain" {
  type        = string
  default     = "ursa.earth"
  description = "Ursa domain name"
}

variable "node_count" {
  default     = 7
  type        = number
  description = "How many testnet nodes to deploy"
}

variable "bootstrap_count" {
  default     = 2
  type        = number
  description = "How many bootstrap nodes to deploy"
}
