resource "helm_release" "ingress_nginx" {
  name       = "nginx-ingress-controller"
  repository = "https://charts.bitnami.com/bitnami"
  chart      = "nginx-ingress-controller"
  namespace  = kubernetes_namespace_v1.ursa.metadata[0].name

  set {
    name  = "service.type"
    value = "LoadBalancer"
  }
  set {
    name  = "service.annotations.kubernetes\\.digitalocean\\.com/load-balancer-id"
    value = digitalocean_loadbalancer.main.id
  }

  depends_on = [
    digitalocean_loadbalancer.main,
  ]
}

resource "kubernetes_ingress_v1" "default_cluster_ingress" {
  depends_on = [
    helm_release.ingress_nginx,
  ]
  metadata {
    name      = "ursa-${var.region}-ingress"
    namespace = kubernetes_namespace_v1.ursa.metadata[0].name
    annotations = {
      "kubernetes.io/ingress.class"          = "nginx"
      "ingress.kubernetes.io/rewrite-target" = "/"
      "ingress.kubernetes.io/ssl-redirect"   = "false"
      "cert-manager.io/cluster-issuer"       = "letsencrypt-${var.lets_encrypt_env}"
    }
  }
  spec {
    dynamic "rule" {
      for_each = toset(var.k8s_domains)
      content {
        host = rule.value
        http {
          path {
            backend {
              service {
                name = kubernetes_service_v1.ursa.metadata[0].name
                port {
                  number = 4069
                }
              }
            }
            path      = "/"
            path_type = "Prefix"
          }
        }
      }
    }
    dynamic "tls" {
      for_each = toset(var.k8s_domains)
      content {
        hosts       = [tls.value]
        secret_name = "ursa-tls"
      }
    }
  }
}