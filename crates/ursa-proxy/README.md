# Ursa Proxy

Please see [example.toml](./example/example.toml) which contains information 
about the parameters that can be configured.

## Admin

While the proxy is running you can send it commands via the admin server.
The admin server has two endpoints:
* `/purge` will purge the cache. 
* `/reload-tls-config` will reload the TLS config. Use this if you want to reload the certificates.

## Metrics

We use [axum_prometheus](https://docs.rs/axum-prometheus/latest/axum_prometheus/) to collect 
HTTP metrics. This data can be accessed using endpoint `/metrics`.