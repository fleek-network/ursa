FROM rust:latest as builder

# Make a fake Rust app to keep a cached layer of compiled crates
# RUN USER=root cargo new app
# WORKDIR /usr/src/app
# COPY ./cli/Cargo.toml ./cli/Cargo.lock ./

# # Needs at least a main.rs file with a main function
# RUN mkdir src && echo "fn main(){}" > src/main.rs

# # Will build all dependent crates in release mode
# RUN --mount=type=cache,target=/usr/local/cargo/registry \
#     --mount=type=cache,target=/usr/src/app/target \
#     cargo build --release

# Copy the rest
COPY . .

# Build (install) the actual binaries
RUN make install

# Runtime image
FROM debian:bullseye-slim

# Run as "app" user
# RUN useradd -ms /bin/bash app

# USER app
# WORKDIR /app

# Install dependencies
RUN apt-get update && apt-get install --no-install-recommends -y build-essential clang

# Get compiled binaries from builder's cargo install directory
COPY --from=builder /usr/local/cargo/bin/ursa /usr/local/bin/ursa

# run ursa node
ENTRYPOINT ["/usr/local/bin/ursa"]
