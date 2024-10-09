# Reorgs

Infinity implements the InfinityVM architecture, key to which is ensuring the fork choice rule abandons blocks that contain invalid coprocessing results.

To achieve these reversion capabilities, we pursue an approach that decouples consensus and execution such that there is a separate consensus chain and execution chain [^note1]. The end result is that the consensus chain will have single slot finality while the execution chain will be subject to roll back within a fraud proof period.

[^note1]: In practice, outside of this section, we refer to execution chain blocks as execution payloads and consensus chain blocks simply as blocks. In this section, we refer to them as separate chains to illustrate how the execution chain can independently be rolled back. For context, the consensus chain blocks contain the execution payload and the root of execution chain state, in addition to the root of consensus state. The execution client also independently propagates execution blocks.

## High level flows

### Notation

- `E` is the execution chain block containing a fraudulent coprocessor result.
- `E -1` is the parent block of `E`.
- `E'` is the replacement for `E` and becomes the canonical child of `E -1`.
- `C` consensus chain block that contains execution payload for `E'`.

### Block Proposing

Steps for a validator to propose a consensus block that rolls back the execution chain:

1) Proposer receives a coprocessor fraud proof via p2p networking. The proof indicates a fraudulent coprocessor result in execution chain block E. 
1) Proposer successfully verifies proof.
1) Proposer starts constructing `C`, including the fraud proof in the block body.
1) Proposer requests the creation of execution payload for `E'` by calling [`engine_forkchoiceUpdated`](https://github.com/ethereum/execution-apis/blob/main/src/engine/cancun.md#engine_forkchoiceupdatedv3) against the Execution Client (EC), specifying `headBlockHash = E -1`.
1) Proposer retrieves the payload for `E'`, and includes the payload in `C`.
1) Proposer finishes constructing `C` and proposes it to the network.

### Block Verification

Steps for a validator to verify a consensus block that rolls back the execution chain:

1) Verifier receives `B`, containing a fraudulent coprocessor result from `E` and execution payload `E'`. 
1) Verifier successfully verifies the proof (or the block is rejected, the proposer is penalized and we exit).
1) Verifier verifies execution payload for `E'` by calling [`engine_newPayload`](https://github.com/ethereum/execution-apis/blob/main/src/engine/cancun.md#engine_newpayloadv3) against the EC, (where the `parentHash = block E -1`).
1) Once the Verifier completes remaining consensus block checks and staking accounting, the consensus chain has monotonically advanced one block. Effectively, the execution chain has been rolled back to not include the fork building on `E` and instead having the head `E'`.
