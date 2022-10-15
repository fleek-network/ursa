#!/bin/bash

# shellcheck disable=SC2086
cd "$(dirname $BASH_SOURCE)" || exit 1
shopt -s expand_aliases; alias ursa="../target/release/ursa"

NODES=3; [[ -n $1 ]] && NODES=$1

# trap ctrl-c and cleanup
trap ctrl_c INT
function ctrl_c() {
        echo "** CTRL-C - Killing child jobs and cleaning up..."
        # kill all children
        jobs -p | xargs kill
        rm ursadb/ -rf
}

printf "Starting Bootstrap node\n"
ursa -c bootstrap.toml &
sleep 2

for n in $(seq $NODES); do
  port=$(bc <<< "$n + 6009"); n=node$n
  # shellcheck disable=SC2086
  ursa -i ${n} -d=test_db/${n} -c node1.toml -s /ip4/127.0.0.1/tcp/${port} &
  sleep 2
done

wait
