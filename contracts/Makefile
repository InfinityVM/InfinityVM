############################# HELP MESSAGE #############################
# Make sure the help command stays first, so that it's printed by default when `make` is called without arguments
.PHONY: help tests
help:
	@grep -E '^[a-zA-Z0-9_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-30s\033[0m %s\n", $$1, $$2}'

-----------------------------: ## 

___CONTRACTS___: ## 

test-contracts: ## runs all forge tests
	forge test -vv

deploy-coprocessor-contracts-to-anvil-and-save-state: ## Deploy coprocessor contracts
	./script/anvil/deploy-coprocessor-save-anvil-state.sh

__CLI__: ##