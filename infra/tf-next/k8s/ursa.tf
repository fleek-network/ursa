resource "kubernetes_deployment" "ursa_node" {
  metadata {
    name      = "ursa-node"
    namespace = kubernetes_namespace.ursa.metadata.0.name
  }
  spec {
    replicas = 2
    selector {
      match_labels = {
        app = "UrsaNode"
      }
    }
    template {
      metadata {
        labels = {
          app = "UrsaNode"
        }
      }
      spec {
        image_pull_secrets {
          name = "dockerconfigjson"
        }

        container {
          image = var.k8s_ursa_docker_image
          name  = "ursa"
          port {
            container_port = 80
          }
        }
      }
    }
  }
}
resource "kubernetes_service" "ursa" {
  metadata {
    name      = "ursa"
    namespace = kubernetes_namespace.ursa.metadata.0.name
  }
  spec {
    selector = {
      app = kubernetes_deployment.ursa_node.spec.0.template.0.metadata.0.labels.app
    }
    type = "NodePort"

    port {
      node_port   = 30201
      port        = 80
      target_port = 80
    }
  }
}