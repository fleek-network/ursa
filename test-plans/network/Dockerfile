FROM rust:1.62-bullseye as builder
WORKDIR /usr/src/testplan

RUN apt-get update && apt-get install -y cmake protobuf-compiler

# Cache dependencies between test runs,
# See https://blog.mgattozzi.dev/caching-rust-docker-builds/
# And https://github.com/rust-lang/cargo/issues/2644

RUN mkdir -p ./plan/src/
RUN echo "fn main() {}" > ./plan/src/main.rs
COPY ./plan/Cargo.lock ./plan/
COPY ./plan/Cargo.toml ./plan/
RUN cd ./plan/ && cargo build --release

COPY . .
RUN cd ./plan/ && cargo build --release && cargo install --path .

FROM debian:bullseye-slim
# Rename ping to name of your crate
COPY --from=builder /usr/local/cargo/bin/ping /usr/local/bin/testplan
EXPOSE 6060
ENV RUST_BACKTRACE=1
ENTRYPOINT ["testplan"]
