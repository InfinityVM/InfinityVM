# Block Lifecycle

In this section, we go through the end-to-end lifecycle of various block-related flows in the Infinity L1 architecture.

### Proposing a block

![propose block](../assets/propose-block.png)

When it's a validator's turn to propose a block, a `getPayload()` Engine API call is made to the execution engine to retrieve the execution payload. The proposed block containing the payload is then gossiped to the rest of the network.

### Processing a block

![process block](../assets/process-block.png)

When a validator receives a block proposal from the proposer, a `newPayload()` Engine API call is made to the execution engine to validate the execution payload in the block.

### Finalizing a block

![finalize block](../assets/finalize-block.png)

When a validator receives a block to finalize from the network, it does two things:

1. It performs the [beacon chain state transition function](https://eth2book.info/capella/part3/transition/) to compute validator updates and makes a `newPayload()` Engine API call to the execution engine to process the execution payload.
1. It sends a `forkChoiceUpdate()` to the execution engine to notify the execution client of the latest block being finalized.
