# UFDP Benchmarking with Terraform

This repository contains the Terraform scripts used to create the infrastructure
for running multiple `ufdp-bench-client` instances.

## Requirements

- Terraform 0.12.x
- AWS account with appropriate permissions

## Usage

1. Initialize the terraform project (and create a ssh key if it's missing)

```bash
make init
```

2. Create the infrastructure

```bash
# optionally, preview the changes
make plan

make apply
```

3. Execute the client on all instances (in parallel)

```bash
# client <socket addr> <num concurrent req> <block size> <file size>
make exec s="client ufdp.server:6969 64 262144 1048576"
```

## Cleanup

To destroy all infrastructure and delete the temp ssh keys:

```bash
make clean
```
