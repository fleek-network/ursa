locals {
  cert_manager_values = {
    installCRDs          = true
    createCustomResource = true
  }
}

resource "helm_release" "cert_manager" {
  name             = "cert-manager"
  repository       = "https://charts.jetstack.io"
  chart            = "cert-manager"
  version          = "v1.10.1"
  wait             = true
  create_namespace = false
  values           = [yamlencode(local.cert_manager_values)]
  depends_on = [
    kubernetes_ingress_v1.default_cluster_ingress,
  ]
}

resource "helm_release" "cluster_issuer" {
  name             = "cluster-issuer"
  chart            = "./cluster-issuer"
  wait             = true
  create_namespace = false
  depends_on = [
    helm_release.cert_manager,
  ]

  set {
    name  = "email"
    value = var.letsencrypt_email
  }
}