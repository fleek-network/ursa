# Ursa Node Tracker

A simple pre-consensus http tracker for Ursa nodes. Provides prometheus http service discovery for metrics.
Nodes register their configuration, and the tracker performs a quick verification, and stores the node info.

The tracker will also perform a simple IP/DNS lookup to assign proper geo labels to the nodes.

## Starting the tracker

```bash
cargo run -r
```

## Registering a node

Nodes can register their configuration by sending a POST request to the tracker's `/register` endpoint.

```bash
curl -X POST \
    http://localhost:4000/register \
    -H "Content-Type: application/json" \
    -d '{
          "id": "12D3KooWLp3tyhzzRjBDbXXyqCUDhvK8xKCFhUEPjgsRZDYzZ62F",
          "addr":"optional.dns.or.ip",
          "storage": 1000000
          "p2p_port":6009,
          "telemetry":true,
          "metrics_port":4070,
    }'
```

## Prometheus service discovery

The tracker provides a `/http_sd` endpoint that can be used by prometheus for discovering nodes.

An example response would look like:

```json
[
  {
    "targets": [ 
      "ip.or.dns:4070"
    ],
    "labels": {
      "id": "12D3KooWLp3tyhzzRjBDbXXyqCUDhvK8xKCFhUEPjgsRZDYzZ62F",
      "country_code": "US",
      "timezone": "America/New_York",
      "geohash": "dre8bq5"
    }
  }
]
```

## Contributing

Pull requests are welcome. For major changes, please open an issue first to discuss what you would like to change.

Please make sure to update tests as appropriate.

## License
[MIT](https://choosealicense.com/licenses/mit/)
