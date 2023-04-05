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

variable "bootstrap_droplet_size" {
  type        = string
  default     = "s-8vcpu-16gb-intel"
  description = "Droplet size identifier for bootstrap nodes"
}

variable "node_droplet_size" {
  type        = string
  default     = "s-8vcpu-16gb-intel"
  description = "Droplet size identifier for provider nodes"
}

variable "dashboard_droplet_size" {
  type        = string
  default     = "s-8vcpu-16gb-intel"
  description = "Droplet size identifier for the dashboard"
}

############
#   Node   #
############

variable "indexer_url" {
  type    = string
  default = "https://dev.cid.contact"
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

variable "ursa_domain" {
  type        = string
  default     = "ursa.zone"
  description = "Ursa domain name"
}

variable "node_count" {
  default     = 5
  type        = number
  description = "How many testnet nodes to deploy"
}

variable "bootstrap_count" {
  default     = 1
  type        = number
  description = "How many bootstrap nodes to deploy"
}

variable "maxmind_account" {
  description = "MaxmindDB Account for geoipupdate"
}

variable "maxmind_key" {
  description = "MaxmindDB secret token for geoipupdate"
}
