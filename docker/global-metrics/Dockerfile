FROM rust:latest as builder

WORKDIR /ursa

RUN apt-get update && apt-get install -y \
    clang \
    cmake \
    libclang-dev \
    protobuf-compiler

COPY . .

ENV RUST_BACKTRACE=1

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/ursa/target \
    cargo build -r --bin ursa-tracker && \
    mv /ursa/target/release/ursa-tracker /ursa/

FROM debian:bullseye-slim

WORKDIR /root

RUN apt-get update && apt-get install -y \
    libcurl4-openssl-dev \
    && apt-get clean \
    && apt-get purge -y \
    && rm -rf /var/lib/apt/lists*

# Get compiled binaries from builder's cargo install directory
COPY --from=builder /ursa/ .

ENTRYPOINT ["./ursa-tracker"]
