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
            container_port = 4069
          }

          port {
            container_port = 6009
            host_port      = 6009
            protocol       = "TCP"
          }

          port {
            container_port = 6009
            host_port      = 6009
            protocol       = "UDP"
          }

          resources {
            limits = {
              cpu    = "0.5"
              memory = "512Mi"
            }
            requests = {
              cpu    = "250m"
              memory = "50Mi"
            }
          }

          liveness_probe {
            http_get {
              path = "/"
              port = 4069

              http_header {
                name  = "X-Custom-Header"
                value = "Awesome"
              }
            }

            initial_delay_seconds = 3
            period_seconds        = 3
          }

        }

        affinity {
          node_affinity {
            required_during_scheduling_ignored_during_execution {
              node_selector_term {
                match_expressions {
                  key      = "doks.digitalocean.com/node-pool"
                  operator = "In"
                  values   = ["ursa-main"]
                }
              }
            }
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