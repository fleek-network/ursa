# Ursa Proof of Delivery

Implementation for the Ursa Fair Delivery Protocol, used to collect and batch proofs of delivery for verified blake3 streams.

## Examples

Request and serve 10GB of content over UFDP

1. Run the server

```sh
cargo run --example server
```

2. Run the client

```sh
cargo run --example client
```

## Benchmarking

### Criterion benchmarks

All UFDP benchmarks can be ran using:

```sh
cargo bench
```

#### Codec

```sh
cargo bench --bench codec
```

#### Encryption

```sh
cargo bench --bench encrypt
```

#### Blake3 Tree

```sh
cargo bench --bench tree
```

#### End to End

UFDP:

```sh
cargo bench --bench e2e
```

HTTP (Hyper 1.0):

> this is used optionally as a comparison, so it's behind a feature flag

```sh
cargo bench --bench e2e --features bench-hyper
```

### UFDP Binary Benchmarking

All methods in the implementation are instrumented with a print line containing the start and end of the call, 
as well as some parameters. This can be enabled with the crate feature `benchmarks`.

#### Client

The benchmarking client will make some concurrent requests to a ufdp server.
File and block size are encoded into the cid bytes, parsed by the server. These are not accurate blake3 hashes, and only used for testing against the bench server.

```sh
# Run 64 concurrent requests for 1MiB of content, in 256KiB blocks
cargo run \
  --bin ufdp-bench-client -- \
  "127.0.0.1:6969" 64 262144 1048576 
```

#### Server

The benchmarking server will accept many requests, also printing the logs for instrumented methods.

```sh
cargo run \
  --features benchmarks \
  --bin ufdp-bench-server \
  > server.out
```

#### Parsing

The parser will collect the output from the log instrumentation, computing the sum/mean/median/std deviation, and outputting a json document.

```sh
cat server.out |\
  cargo run --features benchmarks \
  --bin ufdp-bench-parser \
  > stats.json
```

Statistics can then be traversed using a tool like `jq`:

- Get stats for everything under the `deliver_content` tag

```sh
jq '.params.tag.deliver_content.stats' stats.json
```

- For session id `0`, get stats for the `deliver_content` tag

```sh
jq '.params.sid.0.params.tag.deliver_content.stats' stats.json
```

- Get available param keys for something:

```sh
jq '.params | keys' stats.json
jq '.params.tag.deliver_content | keys' stats.json
```

