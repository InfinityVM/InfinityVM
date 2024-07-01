#!/usr/bin/env bash

set -e

# TODO(Bez): Eventually this will all be in a single Rust workspace, so we'll need
# to refactor the Buf configuration and this script.

cd ./proto; buf generate; cd ..

# Copy over Server (Go) files
cp -r github.com/ethos-works/InfinityVM/server/* server/

# Copy over zkShim (Rust) files
# ...

# Cleanup
rm -rf github.com
