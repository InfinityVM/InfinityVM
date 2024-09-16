
#!/bin/bash

PROJECT_ID=${PROJECT_ID}
SECRET_NAME=${SECRET_NAME}

# Verify that arguments are passed correctly
if [[ -z "$PROJECT_ID" || -z "$SECRET_NAME" ]]; then
  echo "Error: PROJECT_ID or SECRET_NAME not provided."
  exit 1
fi

create_or_update_secret() {
  local instance_name=$1
  local secret_value=$2
  local secret_name="${instance_name}-anvil-accounts"

  # Check if the secret exists
  if gcloud secrets describe $secret_name --project $PROJECT_ID > /dev/null 2>&1; then
    # Update the secret
    echo -n $secret_value | gcloud secrets versions add $secret_name --data-file=-
  else
    # Create the secret
    echo -n $secret_value | gcloud secrets create $secret_name --replication-policy="automatic" --data-file=-
  fi
}

anvil --block-time $BLOCK_TIME --port $PORT > anvil_output.log 2>&1 &

# Wait a few seconds for Anvil to start
sleep 3

if ! grep -q "Listening on" anvil_output.log; then
  echo "Failed to start Anvil. Check the logs."
  cat anvil_output.log
  exit 1
fi

INITIAL_OWNER_KEY=$(grep -A 10 "Private Keys" anvil_output.log | awk 'NR==4 {print $2}')
RELAY_KEY=$(grep -A 10 "Private Keys" anvil_output.log | awk 'NR==5 {print $2}')
COPROCESSOR_OPERATOR_KEY=$(grep -A 10 "Private Keys" anvil_output.log | awk 'NR==6 {print $2}')
OFFCHAIN_SIGNER_KEY=$(grep -A 10 "Private Keys" anvil_output.log | awk 'NR==7 {print $2}')

if [[ -z "$INITIAL_OWNER_KEY" || -z "$RELAY_KEY" || -z "$COPROCESSOR_OPERATOR_KEY" || -z "$OFFCHAIN_SIGNER_KEY" ]]; then
  echo "Failed to retrieve keys from Anvil output."
  exit 1
fi

RELAYER_ADDRESS=$(cast wallet address $RELAY_KEY)
COPROCESSOR_OPERATOR_ADDRESS=$(cast wallet address $COPROCESSOR_OPERATOR_KEY)
OFFCHAIN_SIGNER_ADDRESS=$(cast wallet address $OFFCHAIN_SIGNER_KEY)
INITIAL_MAX_NONCE=0
WRITE_JSON=true
# Deploy contracts using Forge script
forge script script/CoprocessorDeployer.s.sol:CoprocessorDeployer --sig "deployCoprocessorContracts(address relayer, address coprocessorOperator, bool writeJson)" $RELAYER_ADDRESS $COPROCESSOR_OPERATOR_ADDRESS $WRITE_JSON --rpc-url http://localhost:$PORT --private-key $INITIAL_OWNER_KEY --broadcast
forge script script/ClobDeployer.s.sol:ClobDeployer --sig "deployClobContracts(address offchainRequestSigner, uint64 initialMaxNonce, bool writeJson)" $OFFCHAIN_SIGNER_ADDRESS $INITIAL_MAX_NONCE $WRITE_JSON --rpc-url http://localhost:$PORT --private-key $INITIAL_OWNER_KEY --broadcast
forge script script/MockConsumerDeployer.s.sol:MockConsumerDeployer --sig "deployMockConsumerContracts(address offchainSigner, uint64 initialMaxNonce, bool writeJson)" $OFFCHAIN_SIGNER_ADDRESS $INITIAL_MAX_NONCE $WRITE_JSON --rpc-url http://localhost:$PORT --private-key $INITIAL_OWNER_KEY --broadcast

# Set environment variables for contract addresses
export COPROCESSOR_PROXY_ADMIN=$(jq -r '.addresses.coprocessorProxyAdmin' script/output/31337/coprocessor_deployment_output.json)
export JOB_MANAGER=$(jq -r '.addresses.jobManager' script/output/31337/coprocessor_deployment_output.json)
export CLOB_CONSUMER=$(jq -r '.addresses.consumer' script/output/31337/clob_deployment_output.json)
export MOCK_CONSUMER=$(jq -r '.addresses.consumer' script/output/31337/mock_consumer_deployment_output.json)

if [[ -z "$COPROCESSOR_PROXY_ADMIN" || -z "$JOB_MANAGER" || -z "$CLOB_CONSUMER" || -z "$MOCK_CONSUMER" ]]; then
  echo "Failed to retrieve contract addresses from deployment output."
  exit 1
fi

# Prepare the output JSON string
output_json=$(cat <<EOF
{
  "initial_owner_key": "$INITIAL_OWNER_KEY",
  "relayer_key": "$RELAY_KEY",
  "coprocessor_operator_key": "$COPROCESSOR_OPERATOR_KEY",
  "offchain_signer_key": "$OFFCHAIN_SIGNER_KEY",
  "coprocessor_proxy_admin": "$COPROCESSOR_PROXY_ADMIN",
  "job_manager": "$JOB_MANAGER",
  "clob_consumer": "$CLOB_CONSUMER",
  "mock_consumer": "$MOCK_CONSUMER"
}
EOF
)

# Push output JSON to Google Cloud Secret Manager
create_or_update_secret $INSTANCE_NAME "$output_json"
