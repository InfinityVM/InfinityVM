#!/bin/bash

RPC_URL=${RPC_URL:-http://localhost:8545}
CHAIN_ID=${CHAIN_ID:-31337}
BALANCE_ADDRESS=${1:-0xa5889624184923c0c4CFC3499CbeA659e34f7809}

# cd to the directory of this script so that this can be run from anywhere
parent_path=$( cd "$(dirname "${BASH_SOURCE[0]}")" ; pwd -P )
cd "$parent_path"

forge script ../PrintBalance.s.sol:PrintBalance --sig "printBalance(address addr)" $BALANCE_ADDRESS --rpc-url $RPC_URL --chain-id $CHAIN_ID --broadcast -v
