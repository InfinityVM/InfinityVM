############################# HELP MESSAGE #############################
# Make sure the help command stays first, so that it's printed by default when `make` is called without arguments
.PHONY: help tests
help:
	@grep -E '^[a-zA-Z0-9_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-30s\033[0m %s\n", $$1, $$2}'

DEFAULT_RELAYER_ADDRESS=0xaF6Bcd673C742723391086C1e91f0B29141D2381
DEFAULT_COPROCESSOR_OPERATOR_ADDRESS=0xaF6Bcd673C742723391086C1e91f0B29141D2381
DEFAULT_PROGRAM_ID=0x1234
DEFAULT_BALANCE_ADDRESS=0xa5889624184923c0c4CFC3499CbeA659e34f7809

-----------------------------: ## 

___CONTRACTS___: ## 

test-contracts: ## runs all forge tests
	forge test -vv

deploy-coprocessor-contracts-to-anvil-and-save-state: ## Deploy coprocessor contracts and save Anvil state
	./script/cli/deploy-coprocessor-save-anvil-state.sh $(or $(relayerAddress),$(DEFAULT_RELAYER_ADDRESS)) $(or $(coprocessorOperatorAddress),$(DEFAULT_COPROCESSOR_OPERATOR_ADDRESS))

start-anvil-chain-with-coprocessor-deployed: ## Start anvil with coprocessor contracts deployed
	anvil --load-state script/anvil-states/coprocessor-deployed-anvil-state.json --host 0.0.0.0

set-relayer-address: ## Set the relayer address in JobManager
	./script/cli/set-relayer-address.sh $(or $(relayerAddress),$(DEFAULT_RELAYER_ADDRESS))

print-relayer-address: ## Print the relayer address stored in JobManager
	./script/cli/print-relayer-address.sh

set-coprocessor-operator-address: ## Set the coprocessor operator address in JobManager
	./script/cli/set-coprocessor-operator-address.sh $(or $(coprocessorOperatorAddress),$(DEFAULT_COPROCESSOR_OPERATOR_ADDRESS))

print-coprocessor-operator-address: ## Print the coprocessor operator address stored in JobManager
	./script/cli/print-coprocessor-operator-address.sh

request-job: ## Request a job from the coprocessor
	./script/cli/request-job.sh $(or $(programID),$(DEFAULT_PROGRAM_ID)) $(or $(balanceAddress),$(DEFAULT_BALANCE_ADDRESS))

__CLI__: ##
