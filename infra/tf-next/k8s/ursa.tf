resource "kubernetes_daemonset" "ursa_node" {
  metadata {
    name      = "ursa-node"
    namespace = kubernetes_namespace.ursa.metadata.0.name
  }
  spec {
    selector {
      match_labels = {
        app = "UrsaNode"
      }
    }

    strategy {
      rolling_update {
        max_unavailable = 1
      }

      type = "RollingUpdate"
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
              cpu    = "2048"
              memory = "4096Mi"
            }
            # requests = {
            #   cpu    = "250m"
            #   memory = "50Mi"
            # }
          }

          liveness_probe {
            http_get {
              path = "/"
              port = 80

              http_header {
                name  = "X-Custom-Header"
                value = "Awesome"
              }
            }

            initial_delay_seconds = 3
            period_seconds        = 3
          }

          volume_mount {
            name       = "localfs"
            mount_path = "/app/ursa_db"
          }

        }

        volume {
          name = "localfs"
          host_path {
            path = "/opt/ursa"
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

# we might not need the service as long as the ports are exposed on host?
# resource "kubernetes_service" "ursa" {
#   metadata {
#     name      = "ursa"
#     namespace = kubernetes_namespace.ursa.metadata.0.name
#   }
#   spec {
#     selector = {
#       app = kubernetes_daemonset.ursa_node.spec.0.template.0.metadata.0.labels.app
#     }
#     type = "NodePort"

#     port {
#       node_port   = 30201
#       port        = 80
#       target_port = 80
#     }
#   }
# }