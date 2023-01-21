# regional k8s modules
module "fly_ursa_gateway" {
  source      = "./fly"
  regions     = ["yyz", "yul", "ewr", "lax", "ewr", "lhr", "ams"]
  fly_domains = ["gateway.ursa.earth"]
}
