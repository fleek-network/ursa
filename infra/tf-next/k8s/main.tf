terraform {
  required_providers {
    kubernetes = {
      source  = "hashicorp/kubernetes"
      version = ">= 2.16.1"
    }

    digitalocean = {
      source  = "digitalocean/digitalocean"
      version = "~> 2.0"
    }

    helm = {
      source  = "hashicorp/helm"
      version = ">= 2.8.0"
    }
  }
}

resource "digitalocean_kubernetes_cluster" "ursa_cluster" {
  name    = var.cluster_name
  region  = var.region
  version = var.cluster_version

  node_pool {
    name       = "ursa-main"
    size       = var.default_droplet_size
    auto_scale = true
    min_nodes  = var.k8s_min_node_count
    max_nodes  = var.k8s_max_node_count
    tags       = [var.do_tag_ursa_node]
  }
}

resource "digitalocean_kubernetes_node_pool" "bootstrap_pool" {
  cluster_id = digitalocean_kubernetes_cluster.ursa_cluster.id
  auto_scale = false
  name       = "ursa-bootstrap"
  size       = var.default_droplet_size
  node_count = 2
  tags       = [var.do_tag_ursa_bs_node]
}

resource "kubernetes_service" "ursa" {
  metadata {
    name      = "ursa"
    namespace = kubernetes_namespace.ursa.metadata.0.name
  }
  spec {
    selector = {
      app = kubernetes_daemonset.ursa_node.spec.0.template.0.metadata.0.labels.app
    }
    type = "NodePort"

    port {
      name = "api"
      # node_port   = 30201
      port        = 4069
      target_port = 4069
    }
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

provider "helm" {
  kubernetes {
    host  = digitalocean_kubernetes_cluster.ursa_cluster.endpoint
    token = digitalocean_kubernetes_cluster.ursa_cluster.kube_config[0].token
    cluster_ca_certificate = base64decode(
      digitalocean_kubernetes_cluster.ursa_cluster.kube_config[0].cluster_ca_certificate
    )
  }
}