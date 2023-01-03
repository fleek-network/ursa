FROM rust:latest as builder
ARG PROFILE=release
WORKDIR /ursa

RUN apt-get update && apt-get install -y \
    clang \
    cmake \
    libclang-dev

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    cargo install cargo-strip

COPY . .
ENV RUST_BACKTRACE=1

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/ursa/target \
    cargo build --profile $PROFILE --bin ursa-gateway \
    && cargo strip \
    && mv /ursa/target/release/ursa-gateway /ursa/

FROM debian:bullseye-slim

RUN apt-get update && apt-get install -y \
    libcurl4-openssl-dev \
    && apt-get clean \
    && apt-get purge -y \
    && rm -rf /var/lib/apt/lists*

# Get compiled binaries from builder's cargo install directory
COPY --from=builder /ursa/ursa-gateway /usr/local/bin

ENTRYPOINT ["ursa-gateway"]
