FROM rust:1.62-bullseye as builder
WORKDIR /usr/src/testplan

RUN apt-get update && apt-get install -y cmake protobuf-compiler libclang-dev libssl-dev

# Cache dependencies between test runs,
# See https://blog.mgattozzi.dev/caching-rust-docker-builds/
# And https://github.com/rust-lang/cargo/issues/2644

# Clone latest ursa
RUN git clone https://github.com/fleek-network/ursa
# Checkout test-plan branch (we need these changes temporarily)
RUN cd ursa && git checkout feat/test-plan && git pull
RUN cp -r ursa/crates ../crates

RUN mkdir -p ./plan/src/
RUN echo "fn main() {}" > ./plan/src/main.rs
#COPY ./Cargo.lock ./plan/
COPY ./plan/Cargo.toml ./plan/
RUN cd ./plan/ && cargo build --release


COPY . .
RUN cd ./plan/ && cargo build --release && cargo install --path .

FROM debian:bullseye-slim
# Rename ping to name of your crate
COPY --from=builder /usr/local/cargo/bin/network_testplan /usr/local/bin/testplan
EXPOSE 6060
ENV RUST_BACKTRACE=1
ENTRYPOINT ["testplan"]
