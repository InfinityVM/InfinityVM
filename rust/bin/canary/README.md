# Canary OP reth node

This is the ethos chain op executor. Right now its basically a clone of [Reth Alphanet)[https://github.com/paradigmxyz/alphanet].

Specifically, it implements the following EIPS:
 - [EIP-3074](https://eips.ethereum.org/EIPS/eip-3074): `AUTH` and `AUTHCALL` instructions.
 - [EIP-7212](https://eips.ethereum.org/EIPS/eip-7212): Precompile for secp256r1 curve support.
 - [EIP-2537](https://eips.ethereum.org/EIPS/eip-2537): Precompiles for BLS12-381 curve operations.

## Local development

This is adapted from https://reth.rs/run/optimism.html

### Installation

For local configuration, reth nodes can be started with the `--dev` flag.

Install our Canary node:

```sh
cargo install --path bin/canary
```

In some directory outside of the repo install optimism op-node with rethdb-reader

```sh
git clone git@github.com:ethereum-optimism/optimism.git && \
    (cd optimism/op-service/rethdb-reader && cargo build --release) && \ 
    cd optimism/op-node && \
    go build -v -tags rethdb -o ./bin/op-node ./cmd/main.go 
```
You will probably want to add the `op-node/bin` dir to your path. So macos users can add something like: `export PATH="$HOME/ethos/code/optimism/op-node/bin:$PATH"` to their `.zshrc`.

### Running

TODO
- [ ] initial state

## TODO

- [ ] Add section on local development with op node
- [ ] look into using magi
- [ ] look into rethdb-reader - do we need something ecosystem specific?
  - see rethdb build tag in optimism part of book
- [ ] read through engine specs https://github.com/ethereum-optimism/specs/blob/main/specs/protocol/exec-engine.md