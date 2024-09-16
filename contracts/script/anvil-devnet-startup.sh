
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

INITIAL_OWNER_KEY=$(grep -A 10 "Private Keys" anvil_output.log | awk 'NR==2 {print $2}')
RELAY_KEY=$(grep -A 10 "Private Keys" anvil_output.log | awk 'NR==3 {print $2}')
COPROCESSOR_OPERATOR_KEY=$(grep -A 10 "Private Keys" anvil_output.log | awk 'NR==4 {print $2}')
OFFCHAIN_SIGNER_KEY=$(grep -A 10 "Private Keys" anvil_output.log | awk 'NR==5 {print $2}')

if [[ -z "$INITIAL_OWNER_KEY" || -z "$RELAY_KEY" || -z "$COPROCESSOR_OPERATOR_KEY" || -z "$PROXY_ADMIN_KEY" ]]; then
  echo "Failed to retrieve keys from Anvil output."
  exit 1
fi



# Deploy contracts using Forge script
RELAYER_ADDRESS=$(cast wallet address $RELAY_KEY)
COPROCESSOR_OPERATOR_ADDRESS=$(cast wallet address $COPROCESSOR_OPERATOR_KEY)
OFFCHAIN_SIGNER_ADDRESS=$(cast wallet address $PROXY_ADMIN_KEY)
echo "Deploying contracts..."
forge script script/ClobDeployer.s.sol:ClobDeployer --sig "deployClobContracts(address relayer, address coprocessorOperator, address offchainRequestSigner, uint64 initialMaxNonce)" $RELAYER_ADDRESS $COPROCESSOR_OPERATOR_ADDRESS $OFFCHAIN_SIGNER_ADDRESS $INITIAL_MAX_NONCE --rpc-url http://localhost:$PORT --private-key $INITIAL_OWNER_KEY --broadcast



# Prepare the output JSON string
output_json=$(cat <<EOF
{
  "initial_owner_key": "$INITIAL_OWNER_KEY",
  "relayer_key": "$RELAY_KEY",
  "coprocessor_operator_key": "$COPROCESSOR_OPERATOR_KEY",
  "proxy_admin_key": "$PROXY_ADMIN_KEY"
}
EOF
)

# Push output JSON to Google Cloud Secret Manager
create_or_update_secret $INSTANCE_NAME "$output_json"