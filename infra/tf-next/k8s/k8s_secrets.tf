resource "kubernetes_secret" "dockerconfigjson" {
  metadata {
    name = "docker-cfg"
  }

  type = "kubernetes.io/dockerconfigjson"

  data = {
    ".dockerconfigjson" = jsonencode({
      auths = {
        "ghcr.io" = {
          "auth" = base64encode("${var.ghcr_registry_username}:${var.ghcr_registry_password}")
        }
      }
    })
  }
}