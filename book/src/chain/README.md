# Infinity L1

## Overview

Infinity is a high-performance EVM-compatible layer-1 blockchain. Infinity is built with a modular architecture separating execution and consensus.

### Execution

Infinity uses Reth as its high-performance execution engine. Reth is responsible for executing the EVM execution payload in each block, and for gossipping EVM transactions and blocks between execution nodes. This takes advantage of the battle-tested p2p messaging layer built into Reth.

### Consensus

The Infinity L1 uses InfinityBFT, a single-slot finality consensus protocol. This is how nodes gossip and produce blocks, and agree on the contents of each block. InfinityBFT is also modelled after the Ethereum beacon chain (for more info, read the [ETH 2.0 spec](https://eth2book.info/capella/part3/containers/state/)), and maintains the state of the beacon chain. The consensus and execution layer communicate using the [Engine API](https://hackmd.io/@danielrachi/engine_api).

![infinity overview](../assets/infinity-overview.png)

Because Infinity separates execution and consensus, **the chain is able to add custom reorg logic to enshrine offchain compute in the chain's fork choice while still enjoying single-slot finality**. This is because the chain can just reorg the execution layer, while the consensus layer continues as normal.

## Technical Architecture

To better understand the architecture of the Infinity L1 and the optimizations we have made, please read:

- [Key Modules](./modules.md): Descriptions of important modules in the Infinity L1 codebase
- [Block Lifecycle](./lifecycle.md): Overview of the end-to-end lifecycle for proposing blocks, processing blocks, etc.
- [Optimistic Payload Building](./optimistic-payload.md): Performance optimization to prematurely build execution payloads
- [Reorg Logic](./reorg.md): Enshrining offchain compute into the fork choice of the Infinity L1

**Note:** It isn't required to understand any of these sections to build with InfinityVM.
