DOCKER_BUILDKIT=1
COMPOSE_DOCKER_CLI_BUILD=1

version:
	cargo --version
	rustc --version
	@echo "RUSTC_WRAPPER=$${RUSTC_WRAPPER}"
	sccache --show-stats || true

run: version
	cargo run --bin cli

install: version build
	mkdir -p ~/.cargo/bin/
	cp ./target/release/ursa ~/.cargo/bin/

build: version
	cargo build --release --bin ursa

all: build install

docker-build:
	docker build -t ursa -f ./Dockerfile .

docker-run:
	docker run --name ursa-cli -it ursa

compose-build:
	docker-compose -f docker/full-node/docker-compose.yml build

compose-up:
	docker-compose -f docker/full-node/docker-compose.yml up

compose-down:
	docker-compose -f docker/full-node/docker-compose.yml down

# docker run ursa
docker: docker-build docker-run

# Run unit tests
test:
	cargo test --all

# Generate rust docs
doc:
	cargo doc --no-deps --all-features

# Format all sources
fmt: 
	cargo fmt -- --check

# Run clippy on the sources 
clippy:
	cargo clippy -- -D warnings

soft-clippy:
	cargo clippy

# Deep clean
clean:
	cargo clean
	rm -rf target

# Passive check
check:
	cargo check --all --all-targets --all-features

# Trio command for CI/CD
ci:	check fmt soft-clippy
