#!/bin/sh
set -e

echo
echo "Building ursa-gateway node"
echo

docker buildx build \
    --push \
    --platform linux/amd64 \
    --tag fleeknetwork/ursa-gateway:latest \
    -f ./Dockerfile-gateway .