############################# HELP MESSAGE #############################
# Make sure the help command stays first, so that it's printed by default when `make` is called without arguments
.PHONY: help tests
help:
	@grep -E '^[a-zA-Z0-9_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-30s\033[0m %s\n", $$1, $$2}'

DEFAULT_CHAIN_ID=64209
DEFAULT_PRIVATE_KEY=""
DEFAULT_RPC_URL="http://0.0.0.0:8545"
DEFAULT_RELAYER_ADDRESS=0xaF6Bcd673C742723391086C1e91f0B29141D2381
DEFAULT_COPROCESSOR_OPERATOR_ADDRESS=0x184c47137933253f49325B851307Ab1017863BD0
DEFAULT_OFFCHAIN_SIGNER_ADDRESS=0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266
DEFAULT_PROGRAM_ID=programID
DEFAULT_BALANCE_ADDRESS=0xa5889624184923c0c4CFC3499CbeA659e34f7809
DEFAULT_NONCE=1
DEFAULT_INITIAL_MAX_NONCE=0
DEFAULT_RESULT_WITH_METADATA=0000000000000000000000000000000000000000000000000000000000000001fc7fea3498733d20c11a0cf2984b78f88b954525730657cd0283d747479b3d6a00000000000000000000000000000000000000000000000000000000000f424000000000000000000000000000000000000000000000000000000000000000a000000000000000000000000000000000000000000000000000000000000000e0000000000000000000000000000000000000000000000000000000000000000970726f6772616d494400000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000040000000000000000000000000a5889624184923c0c4cfc3499cbea659e34f7809000000000000000000000000000000000000000000000000000000000000000a
DEFAULT_SIGNATURE_ON_RESULT=1b773be023e31ededf7e5b9c1318d7662af71473b308b4bdd72b0d6e474316f83c66bc61e92ed3584ed160f89b0b2f69eb2fade3bc32e3588528e063d8d43b231c
DEFAULT_OFFCHAIN_RESULT_WITH_METADATA=290decd9548b62a8d60345a988386fc84ba6bc95484008f6362f93160ef3e56300000000000000000000000000000000000000000000000000000000000f4240000000000000000000000000000000000000000000000000000000000000008000000000000000000000000000000000000000000000000000000000000000c0000000000000000000000000000000000000000000000000000000000000000970726f6772616d4944000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000400000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000a
DEFAULT_SIGNATURE_ON_OFFCHAIN_RESULT=b3e4536ee1991fec6a009a0df362b8b55a9f2b304d4d94f34a71e81203be3274679ec32fe27c3c7340664bd5c8c7855c8957c371e445b80fe693f381688894fb1c
DEFAULT_JOB_REQUEST=000000000000000000000000000000000000000000000000000000000000000100000000000000000000000000000000000000000000000000000000000f4240000000000000000000000000cf7ed3acca5a467e9e704c703e8d87f634fb0fc900000000000000000000000000000000000000000000000000000000000000a000000000000000000000000000000000000000000000000000000000000000e0000000000000000000000000000000000000000000000000000000000000000970726f6772616d4944000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000000
DEFAULT_SIGNATURE_ON_JOB_REQUEST=b3eaa3283db39bf2de4f91037a7c2544101ad59d8e935c90ab2ae18b09d3988734b3a3bddb24ce0bb52b15e852fbadff36d69dc1e950c3516dee4bf76022a33e1b
DEFAULT_RELAYER_FUNDING_AMOUNT=1 # In Ether

-----------------------------: ## 

___CONTRACTS___: ##

.PHONY: test-contracts
test-contracts: ## runs all forge tests
	forge test -vv

.PHONY: deploy-coprocessor-contracts-to-anvil-and-save-state
deploy-coprocessor-contracts-to-anvil-and-save-state: ## Deploy coprocessor contracts and save Anvil state
	./script/cli/deploy-coprocessor-save-anvil-state.sh $(or $(relayerAddress),$(DEFAULT_RELAYER_ADDRESS)) $(or $(coprocessorOperatorAddress),$(DEFAULT_COPROCESSOR_OPERATOR_ADDRESS)) $(or $(offchainSignerAddress),$(DEFAULT_OFFCHAIN_SIGNER_ADDRESS)) $(or $(initialMaxNonce),$(DEFAULT_INITIAL_MAX_NONCE))

.PHONY: request-job-on-anvil-and-save-state
request-job-on-anvil-and-save-state: ## Request a job from the coprocessor and save Anvil state
	./script/cli/request-job-save-anvil-state.sh $(or $(programID),$(DEFAULT_PROGRAM_ID)) $(or $(balanceAddress),$(DEFAULT_BALANCE_ADDRESS))

.PHONY: start-anvil-chain-with-coprocessor-deployed
start-anvil-chain-with-coprocessor-deployed: ## Start anvil with coprocessor contracts deployed
	anvil --load-state script/anvil-states/coprocessor-deployed-anvil-state.json

.PHONY: start-anvil-chain-with-coprocessor-deployed-and-job-requested
start-anvil-chain-with-coprocessor-deployed-and-job-requested: ## Start anvil with coprocessor contracts deployed and job requested
	anvil --load-state script/anvil-states/coprocessor-deployed-and-job-requested-anvil-state.json

.PHONY: set-relayer-address
set-relayer-address: ## Set the relayer address in JobManager
	./script/cli/set-relayer-address.sh $(or $(relayerAddress),$(DEFAULT_RELAYER_ADDRESS))

.PHONY: print-relayer-address
print-relayer-address: ## Print the relayer address stored in JobManager
	./script/cli/print-relayer-address.sh

.PHONY: set-coprocessor-operator-address
set-coprocessor-operator-address: ## Set the coprocessor operator address in JobManager
	./script/cli/set-coprocessor-operator-address.sh $(or $(coprocessorOperatorAddress),$(DEFAULT_COPROCESSOR_OPERATOR_ADDRESS))

.PHONY: print-coprocessor-operator-address
print-coprocessor-operator-address: ## Print the coprocessor operator address stored in JobManager
	./script/cli/print-coprocessor-operator-address.sh

.PHONY: request-job
request-job: ## Request a job from the coprocessor
	./script/cli/request-job.sh $(or $(programID),$(DEFAULT_PROGRAM_ID)) $(or $(balanceAddress),$(DEFAULT_BALANCE_ADDRESS))

.PHONY: print-job-metadata
print-job-metadata: ## Print the metadata of the job
	./script/cli/print-job-metadata.sh $(or $(nonce),$(DEFAULT_NONCE))

.PHONY: cancel-job
cancel-job: ## Cancel a job
	./script/cli/cancel-job.sh $(or $(nonce),$(DEFAULT_NONCE))

.PHONY: submit-result
submit-result: ## Submit a result for a job (can be generated using crates/scripts/signer.rs)
	./script/cli/submit-result.sh $(or $(resultWithMetadata),$(DEFAULT_RESULT_WITH_METADATA)) $(or $(signature),$(DEFAULT_SIGNATURE_ON_RESULT))

.PHONY: submit-result-for-offchain-job
submit-result-for-offchain-job: ## Submit a result for an offchain job (can be generated using crates/scripts/signer.rs)
	./script/cli/submit-result-for-offchain-job.sh $(or $(resultWithMetadata),$(DEFAULT_OFFCHAIN_RESULT_WITH_METADATA)) $(or $(signatureOnResult),$(DEFAULT_SIGNATURE_ON_OFFCHAIN_RESULT)) $(or $(jobRequest),$(DEFAULT_JOB_REQUEST)) $(or $(signatureOnRequest),$(DEFAULT_SIGNATURE_ON_JOB_REQUEST))

.PHONY: print-balance
print-balance: ## Print the balance of an address in the Consumer contract
	./script/cli/print-balance.sh $(or $(balanceAddress),$(DEFAULT_BALANCE_ADDRESS))

.PHONY: print-job-result
print-job-result: ## Print the result of a job
	./script/cli/print-job-result.sh $(or $(nonce),$(DEFAULT_NONCE))

.PHONY: print-max-nonce
print-max-nonce: ## Print the max nonce on the consumer contract
	./script/cli/print-max-nonce.sh

.PHONY: deploy-coprocessor-contracts
deploy-coprocessor-contracts: ## Deploy coprocessor contracts
	chain=$(or $(chainID),$(DEFAULT_CHAIN_ID))
	mkdir -p script/output/$(chain)
	RPC_URL=$(or $(rpcURL),$(DEFAULT_RPC_URL)) \
	PRIVATE_KEY=$(or $(privateKey),$(DEFAULT_PRIVATE_KEY)) \
	CHAIN_ID=$(chain) \
	RELAYER_FUNDING_AMOUNT=$(or $(relayerFundingAmount), $(DEFAULT_RELAYER_FUNDING_AMOUNT)) 
	sh script/cli/deploy-coprocessor-contracts.sh \
	$(or $(relayerAddress),$(DEFAULT_RELAYER_ADDRESS)) \
	$(or $(operatorAddress),$(DEFAULT_COPROCESSOR_OPERATOR_ADDRESS)) \
	$(or $(offchainSignerAddress),$(DEFAULT_OFFCHAIN_SIGNER_ADDRESS)) \
	$(or $(initialMaxNonce),$(DEFAULT_INITIAL_MAX_NONCE)) \

-__CLI__: ##