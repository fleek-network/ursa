# Ursa Infra

> Deploy a cluster of Ursa nodes using Terraform and k8s.

## Prerequisites

- Install [Terraform](https://developer.hashicorp.com/terraform/tutorials/aws-get-started/install-cli)

## Deploy with Terraform (k8s)

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

## Deploy Legacy Terraform

Set:
- `DIGITALOCEAN_TOKEN=<token_here>`
- `TF_VARS_maxmind_account=<account id>`
- `TF_VARS_maxmind_key=<key>`

> Manually running `terraform import` may be required to populate `tf/terraform.tfstate` corectly

```sh
# Initialize things
make init-tf

# deploy nodes
make apply-tf

# list nodes
make nodes-tf

# ping nodes
make ping-tf

# exec command on nodes
make exec-tf s="cat ~/.ursa/config.toml"

# destroy terraform project
make destroy-tf
```

## Contributing
Pull requests are welcome. For major changes, please open an issue first to discuss what you would like to change.

Please make sure to update tests as appropriate.

## License
[MIT](https://choosealicense.com/licenses/mit/)
