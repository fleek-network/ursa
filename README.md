# Ursa

> Ursa, a decentralized content delivery network.

## Run a node

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
- `--database-path` Database path where store data. This field can be also set on the config file but the cli option will override default and config values.
	- Default value: *ursa_db*. Inside a container, the location will be `/app/ursa_db`.

##### Config file

```toml
mdns = true
relay = false
autonat = false
bootstrap_nodes = ["/ip4/127.0.0.1/tcp/6009"]
swarm_addr = "/ip4/0.0.0.0/tcp/6009"
database_path = "data/ursadb"
```

### Run with Docker

You can run the docker image as a simple container or with docker compose. 

Build the image to create **ursa** or **ursa-dev** docker images: 
```sh
make docker-build
# or
make docker-build-dev
```

Run a node container:
```sh
make docker-run
# or
make docker-run-dev
```

Run or shut down all the infra. This means node + gateway:
```sh
make compose-up
# or
make compose-down
```

### RPC

To access the rpc you can do through the http JSON-RPC api. The endpoint to request is **`/rpc/v0`**. The server can be accessible in port `4060` for local development and in port `80/443` through the gateway (nginx by the moment).

#### JSON-RPC

##### Server Specific Error Response Codes
`-32000`: error while retreiving a block

`-32001`: error while putting the block via rpc

## Contributing
Pull requests are welcome. For major changes, please open an issue first to discuss what you would like to change.

Please make sure to update tests as appropriate.

## License
[MIT](https://choosealicense.com/licenses/mit/)
