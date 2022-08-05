resource "digitalocean_project" "ursa" {
  name        = var.project_name
  description = var.project_description
  purpose     = var.project_purpose
  environment = var.project_stage
}

resource "digitalocean_project_resources" "project_resources" {
  project = digitalocean_project.ursa.id

  for_each = toset(var.regions)

  resources = [
    digitalocean_kubernetes_cluster.ursa_cluster[each.value].urn
  ]
}
