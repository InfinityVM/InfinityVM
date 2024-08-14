#!/bin/bash

RPC_URL=${RPC_URL:-http://localhost:8545}
CHAIN_ID=${CHAIN_ID:-31337}

# cd to the directory of this script so that this can be run from anywhere
parent_path=$( cd "$(dirname "${BASH_SOURCE[0]}")" ; pwd -P )
cd "$parent_path"

forge script ../PrintMaxNonce.s.sol:PrintMaxNonce --sig "printMaxNonce()" --rpc-url $RPC_URL --chain-id $CHAIN_ID --broadcast -v
