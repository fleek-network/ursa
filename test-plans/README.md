# Guide

First, you need to install `testground` and `docker`.
To add a new test-plan, create a new crate in this directory 
and copy the `manifest.toml` and `Dockerfile` into your new crate. 
Configure these files to your test (see comments).
Use `cargo` to build the `Cargo.Lock` file.

Now start the testground daemon:

```bash 
./testground daemon
```

Then, use this command to import the test plan:

```bash
./testground plan import --from ~/Repo/ursa/test-plans/NEW_TEST
```

Use this command to run the test:

```bash
./testground run single --plan=<NAMEOFPLAN>  --testcase=<TESTCASEFROMMANIFEST> --builder=docker:generic --runner=local:docker --instances=<NUMOFINSTANCES>
```

## References

- https://github.com/testground/testground

