# regional k8s modules
module "k8s_apps_ams3" {
  source                = "./k8s"
  region                = "ams3"
  cluster_version       = "1.25.4-do.0"
  cluster_name          = "ursa-${var.region}"
  lets_encrypt_env      = var.lets_encrypt_env
  do_project_id         = digitalocean_project.ursa.id
  k8s_ursa_docker_image = var.k8s_ursa_docker_image
  k8s_domains = [
    "*.ursa.earth",
    "*.testnet.ursa.earth",
  ]
}