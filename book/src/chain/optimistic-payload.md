# Optimistic Payload Building

To improve performance, Infinity optimistically builds execution payloads. This means that when processing a block proposal at height `N`, a validator prematurely requests the execution engine to start building an execution payload for the next block. This improves performance in the case that this validator ends up being the proposer for the next block.

![optimistic payload](../assets/optimistic-payload.png)
