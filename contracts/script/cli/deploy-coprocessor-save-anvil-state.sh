#!/bin/bash

RPC_URL=${RPC_URL:-http://localhost:8545}
PRIVATE_KEY=${PRIVATE_KEY:-0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80}
CHAIN_ID=${CHAIN_ID:-31337}
RELAYER=${1:-0xaF6Bcd673C742723391086C1e91f0B29141D2381}
COPROCESSOR_OPERATOR=${2:-0x184c47137933253f49325B851307Ab1017863BD0}
OFFCHAIN_SIGNER=${3:-0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266}
INITIAL_MAX_NONCE=${4:-0}
WRITE_JSON=${WRITE_JSON:-true}

# cd to the directory of this script so that this can be run from anywhere
parent_path=$( cd "$(dirname "${BASH_SOURCE[0]}")" ; pwd -P )
cd "$parent_path"

# start an anvil instance
anvil --dump-state ../anvil-states/coprocessor-deployed-anvil-state.json &
cd ..
forge script CoprocessorDeployer.s.sol:CoprocessorDeployer --sig "deployCoprocessorContracts(address relayer, address coprocessorOperator, address offchainSigner, uint32 initialMaxNonce, bool writeJson)" $RELAYER $COPROCESSOR_OPERATOR $OFFCHAIN_SIGNER $INITIAL_MAX_NONCE $WRITE_JSON --rpc-url $RPC_URL --private-key $PRIVATE_KEY --chain-id $CHAIN_ID --broadcast -v
# we also do this here to make sure the relayer has funds to submit results to the JobManager contract
cast send $RELAYER --value 10ether --private-key 0x2a871d0798f97d79848a013d4936a73bf4cc922c825d33c1cf7073dff6d409c6
# kill anvil to save its state
pkill anvil
