locals {
  cert_manager_values = {
    installCRDs          = true
    createCustomResource = true
  }
}

resource "helm_release" "cert-manager" {
  name       = "cert-manager"
  repository = "https://charts.jetstack.io"
  chart      = "cert-manager"
  version    = "v1.10.1"
  timeout    = 120
  namespace  = "kube-system"
  values     = [yamlencode(local.cert_manager_values)]
  depends_on = [
    kubernetes_ingress_v1.default_cluster_ingress,
  ]
}

resource "helm_release" "cluster-issuer" {
  name      = "cluster-issuer"
  chart     = "./cluster-issuer"
  namespace = "kube-system"

  set {
    name  = "email"
    value = var.letsencrypt_email
  }

  depends_on = [
    helm_release.cert-manager,
  ]
}