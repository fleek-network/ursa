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

#### Deployment notes.

To deploy a node in DigitalOcean you can take the following notes.

* Basic requirements:
	- Ubuntu 22.04 or later. By the moment, 20.04 is getting errors during the build.
	- 4GB memory is recommended but 2GB should be enough.	- Real domain ownership.


1. Create Droplet and setup the basic for a user to run.
	- Setup user: https://www.digitalocean.com/community/tutorials/initial-server-setup-with-ubuntu-22-04
	- install docker: https://www.digitalocean.com/community/tutorials/how-to-install-and-use-docker-on-ubuntu-22-04
		-  Setup buildkit: https://docs.docker.com/develop/develop-images/build_enhancements/
2. Get a static ip address for your droplet with the available option at the panel.
3. Configure DNS to use your droplet and DigitalOcean name servers.
4. Clone the repo and build the image with `make docker-build`. You can use the `dev` command on the makefile but you also need to change the `infra/ursa/docker-compose.yml` to use `Dockerfile.dev`.
5. Setup TLS.
	- You need to change all the `ursanetwork.local` placeholders for your real domain and run the script in `infra/ursa/init-letsencrypt.sh`.
		- If you have problems during the setup, you can try installing `certbot` locally and then run `sudo certbot certonly --standalone -d domain.com -d www.domain.com` and then move cert and privkey to the correct place.

6. Once you have the DNS configured, TLS certs ready, and the image built, you can run the node by doing `make compose-up` or `docker-compose -f infra/ursa/docker-compose.yml up`


Now you are able to request your node.

```
curl -X POST https://domain.com/rpc/v0 \
 -H "Content-Type: application/json" \
 -d '{"jsonrpc": "2.0", "method": "ursa_get_cid", "params": {"cid": "some-valid-cid"}, "id": 1}'
```
