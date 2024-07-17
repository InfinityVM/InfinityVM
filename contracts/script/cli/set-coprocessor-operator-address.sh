#!/bin/bash

RPC_URL=${RPC_URL:-http://localhost:8545}
CHAIN_ID=${CHAIN_ID:-31337}
PRIVATE_KEY=${PRIVATE_KEY:-0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80}
COPROCESSOR_OPERATOR=${1:-0xaF6Bcd673C742723391086C1e91f0B29141D2381}

# cd to the directory of this script so that this can be run from anywhere
parent_path=$( cd "$(dirname "${BASH_SOURCE[0]}")" ; pwd -P )
cd "$parent_path"

forge script ../SetCoprocessorOperatorAddress.s.sol:SetCoprocessorOperatorAddress --sig "setCoprocessorOperatorAddress(address coprocessorOperator)" $COPROCESSOR_OPERATOR --rpc-url $RPC_URL --private-key $PRIVATE_KEY --chain-id $CHAIN_ID --broadcast -v
