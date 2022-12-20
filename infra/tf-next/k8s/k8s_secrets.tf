resource "kubernetes_secret" "dockerconfigjson" {
  metadata {
    name = "docker-cfg"
  }

  type = "kubernetes.io/dockerconfigjson"

  data = {
    ".dockerconfigjson" = jsonencode({
      auths = {
        "${var.registry_server}" = {
          "auth" = base64encode("${var.ghcr_registry_username}:${var.ghcr_registry_password}")
        }
      }
    })
  }
}