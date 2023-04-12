.PHONY: init build deploy exec

init:
	cd terraform && terraform init

# Runs the client build through the latest rust docker image,
# to ensure compatibility with provisioned clients on the provider.
build:
	docker run \
		--rm --user "$$(id -u)":"$$(id -g)" \
		-v "$$(realpath ../../..)":/usr/share/ursa \
		-w /usr/share/ursa/crates/ursa-pod \
		-v :/usr/local/cargo/registry \
		rust:bullseye \
		cargo build -r --bin client

plan:
	cd terraform && terraform plan

deploy: build
	cd terraform && terraform apply

exec:
	@echo todo
