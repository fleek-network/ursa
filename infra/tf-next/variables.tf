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
  default     = "Major"
  description = "Purpose of the project in DigitalOcean"
}

variable "project_stage" {
  type        = string
  default     = "Development"
  description = "Stage of the project in DigitalOcean"
}

variable "region" {
  type        = string
  default     = "ams3"
  description = "The digital ocean region slug for where to create resources"
}

variable "k8s_domains" {
  type        = list(string)
  default     = ["testnet.ursa.earth"]
  description = "Top level domains to create records and pods for"
}

variable "letsencrypt_email" {
  type        = string
  default     = "major@ursa.earth"
  description = "Let's Encrypt admin email"
}

variable "lets_encrypt_env" {
  type        = string
  default     = "staging"
  description = "Lets Encrypt `staging` or `production`"
}

# For local dev with tfvars
# variable "do_token" {
#   description = "The API token from your Digital Ocean control panel"
#   type        = string
# }

###########
# Images  #
###########
variable "k8s_ursa_docker_image" {
  default     = "ghcr.io/fleek-network/ursa:latest"
  description = "Ursa node docker image"
}
