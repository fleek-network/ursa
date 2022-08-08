terraform {
  required_providers {
    kubernetes = {
      source  = "hashicorp/kubernetes"
      version = ">= 2.0.0"
    }

    digitalocean = {
      source  = "digitalocean/digitalocean"
      version = "2.21.0"
    }
  }
}

resource "digitalocean_kubernetes_cluster" "ursa_cluster" {
  region = var.do_region
  name   = "ursa-${var.do_region}"

  version = "1.23.9-do.0"

  #   tags = ["my-tag"]

  # This default node pool is mandatory
  node_pool {
    name       = "ursa-main"
    size       = var.k8s_droplet_size
    auto_scale = true
    min_nodes  = var.k8s_min_node_count
    max_nodes  = var.k8s_max_node_count
    tags       = ["ursa-main"]
  }

}


# Another node pool in case we need node affinity etc
# resource "digitalocean_kubernetes_node_pool" "app_node_pool" {
#   cluster_id = digitalocean_kubernetes_cluster.kubernetes_cluster.id

#   name = "app-pool"
#   size = "s-2vcpu-4gb" # bigger instances
#   tags = ["applications"]

#   # you can setup autoscaling
#   auto_scale = true
#   min_nodes  = 2
#   max_nodes  = 5
#   labels = {
#     service  = "apps"
#     priority = "high"
#   }
# }


# Kubernetes Provider

resource "digitalocean_project_resources" "project_resources" {
  project = var.do_project_id

  resources = [
    digitalocean_kubernetes_cluster.ursa_cluster.urn
  ]
}

resource "kubernetes_namespace" "ursa" {
  metadata {
    name = "ursa"
  }
}

provider "kubernetes" {
  host  = digitalocean_kubernetes_cluster.ursa_cluster.endpoint
  token = digitalocean_kubernetes_cluster.ursa_cluster.kube_config[0].token
  cluster_ca_certificate = base64decode(
    digitalocean_kubernetes_cluster.ursa_cluster.kube_config[0].cluster_ca_certificate
  )
}

