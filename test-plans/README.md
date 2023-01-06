# Ursa Testground Suite

This repository contains a set of test cases for the Ursa library. The test cases are written in Rust and are executed using the [Testground](https://github.com) framework.

## Running the tests

To run the tests, you need to have the Testground daemon running. You can install it by following the instructions [here](https://docs.testground.ai/getting-started/installation).

```bash
testground daemon
```

Next you'll need to import the test plan into Testground. This will create a new directory in your `$TESTGROUND_HOME/plans` directory.

```bash
testground plan import --from ./ --name ursa
```

Once you have the Testground daemon running, you can run a composition of tests with:

```bash
testground run composition --plan ursa --file ./bootstrap/_compositions/rust.toml
```

> Progess of the tests can be monitored in the daemon's console.

## Test cases

The following test cases are available:
- Data transfer
