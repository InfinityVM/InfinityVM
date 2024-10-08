# Integration Concepts

Affordances of infinityvm for building applications.

## Coprocessing Jobs

InfinityVM enables developers to use expressive offchain compute to build and enhance their EVM applications. Each request for compute to the coprocessor is called a Job. InfinityVM guarantees that the canonical execution chain only contains valid job results. There are two paradigms for requesting jobs: onchain and offchain. The results of coprocessing jobs are submitted onchain and can be configured to callback a smart a contract

The general flow for a Job request is:

1. An app contract or an offchain user requests a compute job from the coprocessor.
2. The coprocessor executes this job and submits the result back to the contract.
3. The app contract can simply use the result from the coprocessor in any of their app logic.

### Onchain

Onchain job requests are initiated by an event emitted from a smart contract. 


### Offchain

Offchain job requests are are triggered by sending a request directly to a coprocessor nodes' (CN) submit job endpoint. The result will be submitted onchain and can also be queried directly from the CN.

![offchain job request](../assets/offchain-request.png)