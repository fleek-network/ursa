# Ursa Infra

> Deploy and run an ursa node, with nginx proxy infront. Setup with cerbot and letencrypt. Certs are auto renewed. 

## Prerequisites

- Install [Docker](https://docs.docker.com/get-docker/)

## Usage

```sh
cd ursa

docker build -t ursa -f ./Dockerfile .
```

## Deploy with Terraform

For local development set `do_token` in files `.tfvars`, `variables.tf`, and `main.tf`, or, set `DIGITALOCEAN_TOKEN=<token_here>`.

```sh
# Format your plans
make fmt

# Download your providers, copy your tfvars
make init

# Layout the plan of which resources will be deployed
make plan

# Create the resources in the plan
make apply
```

## Contributing
Pull requests are welcome. For major changes, please open an issue first to discuss what you would like to change.

Please make sure to update tests as appropriate.

## License
[MIT](https://choosealicense.com/licenses/mit/)