FROM rust:latest as builder

# Install dependencies
WORKDIR /usr/src/app
# RUN apt-get update && apt-get install --no-install-recommends -y build-essential clang
RUN apt-get update && apt-get install -y \
    clang \
    libclang-dev

# Make a fake Rust app to keep a cached layer of compiled crates
RUN USER=root cargo new app

# Copy the whole project
COPY . .

ENV RUST_BACKTRACE=1

# Needs at least a main.rs file with a main function
RUN mkdir src && echo "fn main(){}" > src/main.rs

# Will build all dependent crates in release mode
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/src/app/target \
    apt-get install -y cmake && cargo build --release

# Build (install) the actual binaries
RUN make install

# Runtime image
FROM debian:bullseye-slim

# Run as "app" user
RUN useradd -ms /bin/bash app

USER app
WORKDIR /app

# Get compiled binaries from builder's cargo install directory
COPY --from=builder /usr/local/cargo/bin/ursa /usr/local/bin/ursa

# run ursa node
ENV RUST_LOG=info
ENTRYPOINT ["/usr/local/bin/ursa"]
