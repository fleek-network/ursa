# Ursa Infra

> Deploy and run an ursa node, with nginx proxy infront. Setup with cerbot and letencrypt. Certs are auto renewed. 

## Prerequisites

- Install [Docker](https://docs.docker.com/get-docker/)

## Usage

```sh
cd ursa

docker build -t ursa -f ./Dockerfile .
```

## Deploy

```sh
# Format your plans
terraform fmt

# Download your providers
terraform init

# Layout the plan of which resources will be deployed
terraform plan

# Create the resources in the plan
terraform apply
```

## Contributing
Pull requests are welcome. For major changes, please open an issue first to discuss what you would like to change.

Please make sure to update tests as appropriate.

## License
[MIT](https://choosealicense.com/licenses/mit/)