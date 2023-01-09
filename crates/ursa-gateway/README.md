# Ursa Gateway

`ursa-gateways` provide an HTTP-based service that allows browsers and tools to access `ursa` content.

## Prerequisites
- Install [Rust](https://www.rust-lang.org/)

## Usage

### Build
```bash
$ cargo build -p ursa-gateway -r
```

### Run gateway
```bash
$ target/release/ursa-gateway daemon
```

### Configuration
```bash
$ target/release/ursa-gateway --config <your-config.toml> daemon
```
For details, see [here](./example).

## Contributing
Pull requests are welcome. For major changes, please open an issue first to discuss what you would like to change.

Please make sure to update tests as appropriate.

## License
[MIT](https://choosealicense.com/licenses/mit/)
