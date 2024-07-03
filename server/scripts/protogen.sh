#!/usr/bin/env bash

set -e

echo "Compiling Protobuf files..."

cd ../proto; buf generate; cd ..

# Copy over Server (Go) files
cp -r github.com/ethos-works/infinityvm/server/* server/

# Cleanup
rm -rf github.com
