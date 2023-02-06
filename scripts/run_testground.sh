#!/usr/bin/env bash

# import test plans
if [ ! -d "$HOME/testground/plans/ursa" ]; then
    testground plan import --from ../test-plans/ --name ursa
fi

for plan in "$@"
do
    mkdir -p ../test-plans/$plan/crates
    # copy over the current code
    yes | cp -rf crates/ursa-network test-plans/$plan/crates
    yes | cp -rf crates/ursa-index-provider test-plans/$plan/crates
    yes | cp -rf crates/ursa-store test-plans/$plan/crates
    yes | cp -rf crates/ursa-metrics test-plans/$plan/crates
    yes | cp -f Cargo.toml test-plans/$plan/crates
    
    # overwrite the source in Cargo.toml
    sed -i \
    's|ursa-index-provider = { git = "https://github.com/fleek-network/ursa" }|ursa-index-provider = { path = "crates/ursa-index-provider" }|g' \
    test-plans/$plan/Cargo.toml

    sed -i \
    's|ursa-network = { git = "https://github.com/fleek-network/ursa" }|ursa-network = { path = "crates/ursa-network" }|g' \
    test-plans/$plan/Cargo.toml

    sed -i \
    's|ursa-store = { git = "https://github.com/fleek-network/ursa" }|ursa-store = { path = "crates/ursa-store" }|g' \
    test-plans/$plan/Cargo.toml

    # run test plan
    testground daemon & sleep 5 && testground run composition -f test-plans/$plan/_compositions/rust.toml --wait

    # kill daemon running in background
    kill $!  
done
