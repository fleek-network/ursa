# Ursa

[![website](https://img.shields.io/badge/website-000?style=for-the-badge)](https://fleek.network)&nbsp;
[![discord](https://img.shields.io/badge/discord-333?style=for-the-badge)](https://discord.gg/fleekxyz)&nbsp;
[![twitter](https://img.shields.io/badge/twitter-666?style=for-the-badge)](https://twitter.com/fleek_net)&nbsp;
[![rust-ci](https://img.shields.io/github/actions/workflow/status/fleek-network/ursa/rust.yml?branch=main&label=Tests&style=for-the-badge)](https://github.com/fleek-network/ursa/actions/workflows/rust.yml)&nbsp;
[![docker-build](https://img.shields.io/github/actions/workflow/status/fleek-network/ursa/docker-publish.yml?branch=main&label=Docker%20Build&style=for-the-badge)](https://github.com/fleek-network/ursa/pkgs/container/ursa)&nbsp;

> Ursa, a decentralized content delivery network.

## Run a node

### Run with cli

> Note: Full nodes are intended to run behind a reverse proxy providing ssl and listening on 80/443. See [# Run with Docker Compose](#run-with-docker-compose) for a preconfigured setup.

#### Dependencies

- make
- rust (`^1.65.0`)
- build-essential
- libclang
- cmake
- protoc

#### Instructions

Build and install the latest *HEAD* version:
```sh
make install
```

You can run the node with `ursa` command. This will run the node with default parameters.

#### CLI Flags
- `--config` A toml file containing relevant configurations.
	- Default value: *empty*. 
- `--rpc` Allow rpc to be active or not.
 	- Default value: *true*.
- `--rpc-port` Port used for JSON-RPC communication.
	- Default value: *4069*.

#### CLI Subcommands

- `rpc put` Put a CAR file into the local node
- `rpc get` Get content for a cid from the local node, and save to path

#### Configuration

The default ursa config is loaded from `~/.ursa/config.toml`, but can be overridden using the `--config` flag.

```toml
[network_config]
mdns = false
relay_server = true
autonat = true
relay_client = true
bootstrapper = false
bootstrap_nodes = ["/ip4/127.0.0.1/tcp/6009"]
swarm_addrs = ["/ip4/0.0.0.0/tcp/6009", "/ip4/0.0.0.0/udp/4890/quic-v1"]
database_path = "~/.ursa/data/ursa_db"
keystore_path = "~/.ursa/keystore"
identity = "default"

[provider_config]
domain = "example.domain"
indexer_url = "https://dev.cid.contact"
database_path = "~/.ursa/data/index_provider_db"

[server_config]
port = 4069
addr = "0.0.0.0"
```

### Run with Docker Compose

You can run the full node with some supporting infrastructure through docker-compose. This includes:

- Ursa Node
- Nginx reverse proxy
- Let's Encrypt ssl
- Prometheus Metrics
- Grafana Dashboard

#### Dependencies

- Docker (with Buildkit)
- Docker Compose

#### Instructions

> Make sure to edit [nginx/app.conf](/docker/full-node/data/nginx/app.conf) with your node's domain name, and run [init-letsencrypt.sh](/docker/full-node/init-letsencrypt.sh). Detailed instructions [here](/docker/full-node/README.md)

Build the node and fetch infra images: 

```sh
make compose-build
```

Start up node and infra:

```sh
make compose-up
```

Shut down node and infra:
```sh
make compose-down
```

#### Configuration

By default, the compose will bind-mount the host folder `~/.ursa/` to the node. Any configuration/keys/database files can be located and edited on the host machine at that path. Any changes requires the node to be restarted to take effect

### RPC & HTTP

To access the rpc you can do through the http JSON-RPC api. The endpoint to request is **`/rpc/v0`**. The server can be accessible in port `4069` for local development and in port `80/443` through the reverse proxy (nginx at the moment).

## Contributing
Pull requests are welcome. For major changes, please open an issue first to discuss what you would like to change.

Please make sure to update tests as appropriate.

## License
[MIT](https://github.com/fleek-network/ursa/blob/main/LICENSE-MIT)
[APACHE 2.0](https://github.com/fleek-network/ursa/blob/main/LICENSE-APACHE)
