.PHONY: init build check plan deploy destroy exec

IPS=`jq -rc '.outputs.ip_addrs.value[]' terraform.tfstate.d/UFDP/terraform.tfstate`

all: init build deploy

init:
	terraform init
	[[ -f "benchmark_key" ]] || ssh-keygen -f benchmark_key

# Runs the client build through the latest rust docker image,
# to ensure compatibility with provisioned clients on the provider.
build:
	docker run \
		--rm --user "$$(id -u)":"$$(id -g)" \
		-v "$$(realpath ../..)":/usr/share/ursa \
		-w /usr/share/ursa/crates/ursa-pod \
		-v :/usr/local/cargo/registry \
		rust:bullseye \
		cargo build -r --bin ufdp-bench-client

check:
	terraform validate

plan:
	terraform plan

deploy:
	terraform apply

destroy:
	terraform destroy

exec:
	@for ip in $(IPS); do \
		ssh admin@$$ip -i "benchmark_key" -o StrictHostKeyChecking=no "$s" & \
  done; wait
