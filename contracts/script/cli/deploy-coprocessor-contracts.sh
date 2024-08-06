#!/bin/bash

set -uxo pipefail

RPC_URL=${RPC_URL:-http://0.0.0.0:8545}
PRIVATE_KEY=${PRIVATE_KEY:-}
CHAIN_ID=${CHAIN_ID:-64209}
RELAYER_ADDRESS=${1:-0xaF6Bcd673C742723391086C1e91f0B29141D2381}
COPROCESSOR_OPERATOR_ADDRESS=${2:-0x184c47137933253f49325B851307Ab1017863BD0}
OFFCHAIN_SIGNER_ADDRESS=${3:-0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266}
RELAYER_FUNDING_AMOUNT=${RELAYER_FUNDING_AMOUNT:-1}
WRITE_JSON=${WRITE_JSON:-true}

# cd to the directory of this script so that this can be run from anywhere
parent_path=$( cd "$(dirname "${BASH_SOURCE[0]}")" ; pwd -P )
cd "$parent_path"
cd ..

forge script CoprocessorDeployer.s.sol:CoprocessorDeployer --sig "deployCoprocessorContracts(address relayer, address coprocessorOperator, address offchainSigner, bool writeJson)" $RELAYER_ADDRESS $COPROCESSOR_OPERATOR_ADDRESS $OFFCHAIN_SIGNER_ADDRESS $WRITE_JSON --rpc-url $RPC_URL --private-key $PRIVATE_KEY --chain-id $CHAIN_ID --broadcast -v
# we also do this here to make sure the relayer has funds to submit results to the JobManager contract
cast send $RELAYER_ADDRESS --value ${RELAYER_FUNDING_AMOUNT}ether --private-key $PRIVATE_KEY --rpc-url $RPC_URL
