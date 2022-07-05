version:
	cargo --version
	rustc --version
	@echo "RUSTC_WRAPPER=$${RUSTC_WRAPPER}"
	sccache --show-stats || true

install: version
	cargo install --locked --path cli --force

build: version
	cargo build --release --bin ursa

all: build install

docker-build:
	docker build -t ursa -f ./Dockerfile .

docker-run:
	docker run --name ursa-cli -it ursa
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