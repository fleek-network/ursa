resource "digitalocean_domain" "k8s_domains" {
  for_each = toset(var.k8s_domains)
  name     = each.value
}

resource "digitalocean_record" "a_records" {
  for_each = toset(var.k8s_domains)
  domain   = each.value
  type     = "A"
  ttl      = 60
  name     = "@"
  value    = digitalocean_loadbalancer.main.ip
  depends_on = [
    digitalocean_domain.k8s_domains,
    kubernetes_ingress_v1.default_cluster_ingress
  ]
}

resource "digitalocean_record" "cname_redirects" {
  for_each = toset(var.k8s_domains)
  domain   = each.value
  type     = "CNAME"
  ttl      = 60
  name     = "www"
  value    = "@"
  depends_on = [
    digitalocean_domain.k8s_domains,
  ]
}