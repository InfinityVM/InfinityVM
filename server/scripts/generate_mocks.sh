#!/bin/bash

echo "Generating Test Mocks"
mockgen -source=pkg/eth/eth_client.go  -destination=pkg/testutils/eth_mocks.go -package=testutil
