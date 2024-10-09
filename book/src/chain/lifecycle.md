# Block Lifecycle

This section includes details on the end-to-end lifecycle of various block-related flows in the Infinity L1 architecture, such as proposing and processing a block.

Infinity is built with an event-based architecture. The general block lifecycle looks like this:

1. Consensus triggers some event to be emitted, such as when it's a validator's turn to propose a block.
2. This event is handled by making calls to the execution engine, which communicates with the execution client through the Engine API.
3. If relevant, the execution client returns some data (such as validator updates) to the consensus layer.

We go through a few end-to-end flows across this lifecycle below:

### Proposing a block

![propose block](../assets/propose-block.png)

When it's a validator's turn to propose a block, an event is emitted which triggers a `getPayload()` Engine API call to retrieve the execution payload from the execution engine. The proposed block is then gossiped to the rest of the network.

### Processing a block

![process block](../assets/process-block.png)

When a validator receives a beacon block from the proposer, a `newPayload()` Engine API call is made to the execution engine to validate the execution payload in the block.

### Finalizing a block

![finalize block](../assets/finalize-block.png)

When a validator receives a final beacon block from the network, it does two things:

1. It performs the state transition function and makes a `newPayload()` Engine API call to the execution engine to process the execution payload and compute a list of validator updates.
2. It sends a `forkChoiceUpdate()` to the execution engine to notify the execution client of the latest block being finalized.
