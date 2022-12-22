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

docker-run:
	docker run --name ursa-cli \
		-p 4069:4069 -p 6009:6009 -p 8070:8070 \
		-v ~/.ursa:/root/.ursa -it ursa

compose-up:
	docker-compose -f infra/ursa/docker-compose.yml up

compose-down:
	docker-compose -f infra/ursa/docker-compose.yml down

#	docker run ursa

docker: docker-build docker-run

# Run unit tests
test:
	cargo test --all

# Run unit tests
test-network:
	cargo test -p ursa-network

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

ci:	check fmt soft-clippy
