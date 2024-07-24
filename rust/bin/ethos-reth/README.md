# Ethos Reth

Ethos Reth is the Ethos Chain OP execution client. Right now its basically a clone of [Reth Alphanet][5]([blog post][4]).

Ethos Reth implements the following EIPS:
 - [EIP-3074](https://eips.ethereum.org/EIPS/eip-3074): `AUTH` and `AUTHCALL` instructions.
 - [EIP-7212](https://eips.ethereum.org/EIPS/eip-7212): Precompile for secp256r1 curve support.
 - [EIP-2537](https://eips.ethereum.org/EIPS/eip-2537): Precompiles for BLS12-381 curve operations.

## Overview

A normal Ethereum node is composed of a consensus client and an execution client. Common consensus clients in Ethereum are Prysm, Lighthouse, and Teku. The most common execution client in Ethereum is Geth. You can read more [here][1] about clients on Ethereum.

For the OP stack, we can make an analogous distinction between the execution client and the consensus client. Their are 3 main parts that any verifier or sequencer will need to run an OP L2.

- L2 Execution Client. Ethos Reth is our execution client. Our client is based on Reth and uses the Reth Optimism features. It stores the chain state and serves the Engine API to the Consensus Client.
- L2 Consensus Client: OP Node is the canonical OP consensus client and what we plan to use. It takes L2 txns (L2 smart contract interaction) + L1 blocks (deposit/withdraw events) and derives L2 blocks. This can be run either in validator mode or sequencing mode.
- L1 Execution Client. This could be something like Reth or Geth. The L1 execution client needs to be keeping up with the tip of the L1 chain.

For those developing against this portion of the stack we recommend reading the [OP spec overview][2]. Other notable reading includes the [block derivation spec][3]

## Local development

### Installation

For local configuration, reth nodes can be started with the `--dev` flag.

Install our Ethos Reth node:

```sh
cargo install --path bin/ethos-reth
```

In some directory outside of the repo install optimism op-node with rethdb-reader

```sh
git clone git@github.com:ethereum-optimism/optimism.git && \
    (cd optimism/op-service/rethdb-reader && cargo build --release) && \ 
    cd optimism/op-node && \
    go build -v -tags rethdb -o ./bin/op-node ./cmd/main.go 
```

You will probably want to add the `op-node/bin` dir to your path. So macos users can add something like: `export PATH="$HOME/ethos/code/optimism/op-node/bin:$PATH"` to their `.zshrc`. Make sure to start a new shell or run `source .zshrc`.

### Running just execution for development

First, setup a reth "data directory" for the ethos-reth dev node:

```sh
mkdir -p "$HOME/.config/ethos/networks/ethos-reth-dev/reth"
```

Start the Ethos Reth node in development "mining" mode:

```sh
ethos-reth node
    --dev \
    --datadir "$HOME/.config/ethos/networks/ethos-reth-dev/reth"
```

### Running Consensus Client + Execution

TODO: https://github.com/Ethos-Works/InfinityVM/issues/69

## Acknowledgements

- This work is entirely based off of [Reth][6] and the [Alphanet node][5].

[1]: https://ethereum.org/en/developers/docs/nodes-and-clients/client-diversity/
[2]: https://specs.optimism.io/protocol/overview.html
[3]: https://github.com/ethereum-optimism/specs/blob/main/specs/protocol/derivation.md
[4]: https://www.paradigm.xyz/2024/04/reth-alphanet
[5]: https://github.com/paradigmxyz/alphanet
[6]: https://github.com/paradigmxyz/reth