FROM rust:1.65-bullseye as builder
WORKDIR /usr/src/testplan

RUN apt-get update && apt-get install -y cmake protobuf-compiler libclang-dev libssl-dev

# Cache dependencies between test runs,
# See https://blog.mgattozzi.dev/caching-rust-docker-builds/
# And https://github.com/rust-lang/cargo/issues/2644

COPY ./plan/ ./plan
RUN cd ./plan/ && cargo build --release

ARG BINARY_NAME
RUN cd ./plan/  \
    && cargo build --bin=${BINARY_NAME} --release \
    && mv /usr/src/testplan/plan/target/release/${BINARY_NAME} /usr/local/bin/testplan

FROM debian:bullseye
RUN apt-get update && apt-get install -y libcurl4-openssl-dev
COPY --from=builder /usr/local/bin/testplan /usr/local/bin/testplan
EXPOSE 6060
ENV RUST_BACKTRACE=1
ENTRYPOINT ["testplan"]
