version:
	cargo --version
	rustc --version
	@echo "RUSTC_WRAPPER=$${RUSTC_WRAPPER}"
	sccache --show-stats || true

run: version
	cargo run --bin cli

install: version
	cargo install --locked --path crates/ursa --force

build: version
	cargo build --release --bin ursa

all: build install

docker-build:
	docker build -t ursa -f ./Dockerfile .

docker-build-dev:
	docker build -t ursa-dev -f ./Dockerfile.dev --progress tty .

docker-run-dev:
	docker run --name ursa-dev -it ursa-dev

docker-run:
	docker run --name ursa-cli -it ursa

compose-up:
	docker-compose -f infra/ursa/docker-compose.yml up

compose-down:
	docker-compose -f infra/ursa/docker-compose.yml down

#	docker run ursa

docker: docker-build docker-run

# Run unit tests
test:
	cargo test --all

# Generate rust docs
doc:
	cargo doc --no-deps

# Format all sources
fmt: 
	cargo fmt -- --check

# Run clippy on the sources 
clippy:
	cargo clippy --locked -- -D warnings

# Deep clean
clean:
	cargo clean
	rm -rf target

# Passive check
check:
	cargo check --all --all-targets --all-features
	cargo fmt -- --check
	cargo clippy --locked -- -D warnings
