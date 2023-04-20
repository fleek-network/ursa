# Ursa Proxy

Please see [example.toml](./example/example.toml) which contains information 
about the parameters that can be configured.

## Admin

While the proxy is running you can send it commands via the admin server.
The admin server has two endpoints:
* `/purge` will purge the cache. 
* `/reload-tls-config` will reload the TLS config. Use this if you want to reload the certificates.
* `/metrics` is for accessing HTTP metrics stored via prometheus recorder.
