locals {
  cert_manager_values = {
    installCRDs          = true
    createCustomResource = true
    global = {
      leaderElection = {
        namespace = "cert-manager"
      }
    }
  }
}

resource "kubernetes_namespace" "cert_manager" {
  metadata {
    name = "ursa-cert-manager"
  }
}

resource "helm_release" "cert_manager" {
  name             = "cert-manager"
  repository       = "https://charts.jetstack.io"
  chart            = "cert-manager"
  version          = "v1.10.1"
  namespace        = kubernetes_namespace.cert_manager.metadata[0].name
  wait             = true
  atomic           = true
  create_namespace = false
  values           = [yamlencode(local.cert_manager_values)]
  depends_on = [
    kubernetes_ingress.default_cluster_ingress,
  ]
}

resource "helm_release" "cluster_issuer" {
  name             = "cluster-issuer"
  chart            = "${path.module}/cluster-issuer"
  namespace        = kubernetes_namespace.cert_manager.metadata[0].name
  wait             = true
  atomic           = true
  create_namespace = false
  depends_on = [
    helm_release.cert_manager,
  ]

  set {
    name  = "email"
    value = var.lets_enrypt_email
  }
}