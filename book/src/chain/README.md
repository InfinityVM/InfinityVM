# Infinity L1

## Overview

Infinity is a high-performance EVM-compatible layer-1 blockchain with real-time coprocessing. The network is built with a modular architecture separating consensus and execution.

### Consensus

The Infinity L1 uses InfinityBFT, a consensus protocol with single-slot finality. The consensus protocol is responsible for validator selection, gossiping blocks, and agreeing on the contents of each block. The consensus layer is also inspired by the Ethereum beacon chain (for more info, read the [ETH 2.0 spec](https://eth2book.info/capella/part3/containers/state/)), and maintains the beacon chain's state. For this reason, we refer to *consensus layer* and *beacon chain* interchangeably in this doc. The consensus and execution layer communicate using the [Engine API](https://hackmd.io/@danielrachi/engine_api).

### Execution

Infinity uses [Reth](https://github.com/paradigmxyz/reth) as its high-performance execution engine via the [Engine API](https://hackmd.io/@danielrachi/engine_api). Reth is responsible for executing the EVM execution payload in each beacon block, and for the EVM mempool and gossiping payloads between nodes. By using Reth, we can take advantage of all its battle-tested code, and benefit from planned future features such as parallel EVM execution and Reth's goal of [1 gigagas per second](https://www.paradigm.xyz/2024/04/reth-perf).

![infinity overview](../assets/infinity-overview.png)

By separating consensus and execution, **Infinity is able to rollback the execution layer to enshrine offchain compute in the chain's fork choice, while still enjoying single-slot finality on the beacon chain state**. This means that beacon chain state like staking, slashing, etc., will not rollback (it will have single-slot finality) even if Infinity rolls back the execution layer state. This is because the execution layer can be reorged via the Engine API, while the consensus layer continues to move forward.

## Technical Architecture

To better understand the architecture of the Infinity L1 and the optimizations we have made, please read:

- [<u>Block Lifecycle</u>](./lifecycle.md): Overview of the end-to-end lifecycle for proposing blocks, processing blocks, etc.
- [<u>Optimistic Payload Building</u>](./optimistic-payload.md): Performance optimization to prematurely build execution payloads.
- [<u>Fork Choice</u>](./fork-choice.md): Enshrining offchain compute into the fork choice of the Infinity L1.
- [<u>DA</u>](./da.md): DA in Infinity L1 blocks to support offchain compute.

**Note:** It isn't required to understand any of these sections to build with InfinityVM.
