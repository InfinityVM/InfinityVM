#!/bin/bash

RPC_URL=${RPC_URL:-http://localhost:8545}
CHAIN_ID=${CHAIN_ID:-31337}
NONCE=${1:-1}

# cd to the directory of this script so that this can be run from anywhere
parent_path=$( cd "$(dirname "${BASH_SOURCE[0]}")" ; pwd -P )
cd "$parent_path"

forge script ../PrintJobMetadata.s.sol:PrintJobMetadata --sig "printJobMetadata(uint32 nonce)" $NONCE --rpc-url $RPC_URL --chain-id $CHAIN_ID --broadcast -v
