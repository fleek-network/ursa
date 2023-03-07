################
#   Project    #
################

variable "regions" {    
  type        = list(string)
  default     = ["yyz", "yul", "ewr", "lax", "ewr", "lhr", "ams"]
  description = "Default regions for the cluster of gateway nodes"
}

variable "fly_domains" {
  type        = list(string)
  default     = ["gateway.ursa.earth"]
  description = "Top level domains for the cluster of gateway nodes"
}
