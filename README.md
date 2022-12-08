# Ursa

Ursa, a decentralized content delivery network.

## Run a node

### Build Dependencies

> If docker is used, no dependencies are required other than `make`
- (optional) docker
- make
- rust (`^1.65.0`)
- build-essential
- libclang
- cmake
- protoc

### Run with cli

Build and install the latest *HEAD* version:
```sh
make install
```

You can run the node with `ursa` command. This will run the node with default parameters.

##### CLI Commands
- `--config` A toml file containing relevant configurations.
	- Default value: *empty*. 
- `--rpc` Allow rpc to be active or not.
 	- Default value: *true*.
- `--rpc-port` Port used for JSON-RPC communication.
	- Default value: *4069*.


##### Config file

```toml
[network_config]
mdns = false
relay_server = true
autonat = true
relay_client = true
bootstrapper = false
bootstrap_nodes = ["/ip4/127.0.0.1/tcp/6009"]
swarm_addr = "/ip4/0.0.0.0/tcp/6009"
database_path = "~/.ursa/data/ursa_db"
identity = "default"
keystore_path = "~/.ursa/keystore"


[provider_config]
local_address = "0.0.0.0"
port = 8070
domain = "provider.ursa.earth"
indexer_url = "https://dev.cid.contact"
database_path = "~/.ursa/data/index_provider_db"

[metrics_config]
port = "4070"
api_path = "/metrics"

[server_config]
port = 4069
addr = "0.0.0.0"
```

### Run with Docker

You can run the docker image as a simple container or with docker compose. 

Build the image to create **ursa** docker images: 

```sh
make docker-build
```

Run a node container:
```sh
make docker-run
```

Run or shut down all the infra. This means node + gateway:
```sh
make compose-up
# or
make compose-down
```

### RPC

To access the rpc you can do through the http JSON-RPC api. The endpoint to request is **`/rpc/v0`**. The server can be accessible in port `4060` for local development and in port `80/443` through the gateway (nginx by the moment).

## Contributing
Pull requests are welcome. For major changes, please open an issue first to discuss what you would like to change.

Please make sure to update tests as appropriate.

## License
[MIT](https://github.com/fleek-network/ursa/blob/main/LICENSE-MIT)
[APACHE 2.0](https://github.com/fleek-network/ursa/blob/main/LICENSE-APACHE)
