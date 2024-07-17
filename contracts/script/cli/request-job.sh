#!/bin/bash

RPC_URL=${RPC_URL:-http://localhost:8545}
CHAIN_ID=${CHAIN_ID:-31337}
PRIVATE_KEY=${PRIVATE_KEY:-0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80}
PROGRAM_ID=${1:-"programID"}
BALANCE_ADDR=${2:-0xa5889624184923c0c4CFC3499CbeA659e34f7809}

# cd to the directory of this script so that this can be run from anywhere
parent_path=$( cd "$(dirname "${BASH_SOURCE[0]}")" ; pwd -P )
cd "$parent_path"

# Convert the PROGRAM_ID to hex format
PROGRAM_ID_HEX=$(echo -n $PROGRAM_ID | xxd -pu | tr -d '\n')

forge script ../RequestJob.s.sol:RequestJob --sig "requestJob(bytes calldata programID, address balanceAddr)" $PROGRAM_ID_HEX $BALANCE_ADDR --rpc-url $RPC_URL --private-key $PRIVATE_KEY --chain-id $CHAIN_ID --broadcast -v
