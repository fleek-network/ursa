resource "kubernetes_namespace" "nginx_ingress" {
  metadata {
    name = "nginx-ingress"
  }
}

resource "helm_release" "ingress_nginx" {
  name             = "ingress-nginx"
  chart            = "ingress-nginx"
  namespace        = kubernetes_namespace.nginx_ingress.metadata[0].name
  repository       = "https://kubernetes.github.io/ingress-nginx"
  wait             = true
  atomic           = true
  create_namespace = false

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

resource "kubernetes_ingress" "default_cluster_ingress" {
  depends_on = [
    helm_release.ingress_nginx,
  ]
  metadata {
    name      = "ursa-${var.region}-ingress"
    namespace = kubernetes_namespace.cert_manager.metadata.0.name
    annotations = {
      "kubernetes.io/ingress.class"          = "nginx"
      "ingress.kubernetes.io/rewrite-target" = "/"
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
              service_name = "ursa-${rule.value}-service"
              service_port = 4069
            }
            path = "/"
          }
        }
      }
    }
    dynamic "tls" {
      for_each = toset(var.k8s_domains)
      content {
        hosts       = [tls.value]
        secret_name = "ursa-${tls.value}-tls"
      }
    }
  }
}