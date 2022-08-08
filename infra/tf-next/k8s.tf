# regional k8s modules
module "k8s_apps_ams3" {
  source                 = "./k8s"
  do_region              = "ams3"
  do_project_id          = digitalocean_project.ursa.id
  ghcr_registry_username = var.ghcr_registry_username
  ghcr_registry_password = var.ghcr_registry_password
  k8s_ursa_docker_image  = var.k8s_ursa_docker_image
}
