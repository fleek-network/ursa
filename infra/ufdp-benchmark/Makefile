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

clean:
	terraform destroy
	rm benchmark_key*

sync-client:
	@for ip in $(IPS); do \
		echo "$$ip:"; \
		scp -i benchmark_key ../../target/release/ufdp-bench-client admin@$$ip:client; \
		ssh admin@$$ip -i benchmark_key "sudo mv client /usr/bin"; \
	done;

exec:
	@for ip in $(IPS); do \
		ssh admin@$$ip -i "benchmark_key" -o StrictHostKeyChecking=no "$s" 2>&1 |\
		{ while read i; do [[ -n "$$i" ]] && printf "%b" "[$$ip] $$i\n"; done } & \
  done; wait
