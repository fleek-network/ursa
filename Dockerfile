FROM rust:latest AS rust

# setup build environment
WORKDIR /app
RUN cargo install cargo-chef
RUN cargo install cargo-strip
RUN apt-get update && apt-get install -y \
    clang \
    cmake \
    libclang-dev \
    protobuf-compiler

FROM rust as planner

COPY . .
RUN cargo chef prepare  \
    --recipe-path recipe.json

FROM rust as builder

# build dependencies
COPY --from=planner /app/recipe.json .
RUN cargo chef cook \
    --recipe-path recipe.json \
    --bin ursa --release

# build application
COPY . .
ENV RUST_BACKTRACE=1
RUN cargo build --bin ursa --release && \
    cargo strip && \
    mv ./target/*/ursa .

FROM debian:bullseye-slim

RUN apt-get update && apt-get install -y libcurl4-openssl-dev \
    && apt-get clean && apt-get purge -y \
    && rm -rf /var/lib/apt/lists*

# Get compiled binaries from builder's cargo install directory
COPY --from=builder /app/ursa /usr/local/bin

# run ursa node
ENTRYPOINT ["ursa"]
