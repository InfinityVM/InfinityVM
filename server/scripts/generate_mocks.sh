#!/bin/bash

echo "Generating Test Mocks"
mockgen -source=pkg/eth/eth_client.go  -destination=pkg/testutil/eth_mocks.go -package=testutil
mockgen -source=pkg/relayer/coordinator.go  -destination=pkg/testutil/coordinator_mocks.go -package=testutil
