# ursa

## Run a node

### Run with cli

To run a node through CLI you need to build and install it. You can run `make install` and this will build and install the latest *HEAD* version. 

You can run the node with `ursa` command. This will run the node with default parameters.

- `--config` A toml file containing relevant configurations.
	- Default value: *empty*. 
- `--rpc` Allow rpc to be active or not.
 	- Default value: *true*.
- `--rpc-port` Port used for JSON-RPC communication.
	- Default value: *4069*.
- `--database-path` Database path where store data. This field can be also set on the config file but the cli option will override default and config values.
	- Default value: *ursa_db*. Inside a container, the location will be `/app/ursa_db`.

###### Config file

```toml
mdns = true
relay = false
autonat = false
bootstrap_nodes = ["/ip4/127.0.0.1/tcp/6009"]
swarm_addr = "/ip4/0.0.0.0/tcp/6009"
database_path = "data/ursadb"
```

### Run with Docker

You can run the docker image as a simple container or with docker compose. anyways you need to build the image first with `make docker-build` and `make docker-build-dev` to create **ursa** and **ursa-dev** docker images in any case.

1. `make docker-run / docker-run-dev` to run a node container.
2. `make compose-up / compose-down` to run or shut down all the infra. This means node + gateway.

Unless you want to loose the data once the container is restarted/dropped, share a volume with the database location. Default is always in `/app/ursa_db`.

### Accessing rpc server

To access the rpc you can do through the http JSON-RPC api. The endpoint to request is **`/rpc/v0`**. The server can be accessible in port `4060` for local development and in port `80/443` through the gateway (nginx by the moment).
