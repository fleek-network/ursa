# Ursa Proof of Delivery

Implementation for the Ursa Fair Delivery Protocol, used to collect and batch proofs of delivery for verified blake3 streams.

## Examples

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

#### Encrypt

```sh
cargo bench --bench encrypt
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
