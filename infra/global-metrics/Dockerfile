FROM rust:latest as builder

WORKDIR /usr/src/app

RUN apt-get update && apt-get install -y \
    clang \
    cmake \
    libclang-dev \
    protobuf-compiler

COPY . .

ENV RUST_BACKTRACE=1

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/src/app/target \
    cargo build -r --bin ursa-tracker && \
    mv /usr/src/app/target/release/ursa-tracker /usr/src/app/

FROM debian:bullseye-slim

RUN apt-get update && apt-get install -y \
    libcurl4-openssl-dev \
    && apt-get clean \
    && apt-get purge -y \
    && rm -rf /var/lib/apt/lists*

# Get compiled binaries from builder's cargo install directory
COPY --from=builder /usr/src/app/ /

# run ursa node
ENV RUST_LOG=info
ENTRYPOINT ["/ursa-tracker"]
