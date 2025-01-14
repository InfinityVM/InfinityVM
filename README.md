![infinityvm banner](./book/src/assets/infinityvm-banner.png)

## A new blockchain architecture with native offchain compute

InfinityVM enables developers to use expressive offchain compute alongside the EVM to create new types of applications. Applications can run as fully offchain servers that verifiably settle to the InfinityVM base layer.

## Docs

Consult the [InfinityVM Book](./book) for detailed docs on:

- How to build an app with InfinityVM
- InfinityVM contracts
- InfinityVM coprocessor node
- Infinity L1 architecture

## Getting Started

### Prerequisites

Instructions on how to install the prerequisites are [here](./CONTRIBUTING.md#install-tools).

### Build and run tests

Build

```sh
cd contracts && forge build && cd ..
cargo build
```

Unit tests

```sh
cargo test
```

E2E tests

```sh
make test-all
```

## Directory Structure

The following are some of the more important directories in the InfinityVM repository:

```shell
.
├── contracts              // Onchain business logic
├── crates                 // InfinityVM coprocessor
│   ├── coproc             // Core logic of coprocessor
│   ├── db                 // Database for coprocessor node
│   ├── scripts            // Helper scripts for testing a remote node, generating signed requests/results, etc.
│   ├── sdk                // Shared types and functions useful for interacting with coprocessor
│       ├── abi            // ABI-encoded types
│       ├── test-utils     // Utils for writing tests
│   ├── zkvm-executor      // zkVM interface used by coprocessor
│   └── zkvm               // zkVM trait and implementations      
├── examples               // Example apps built with InfinityVM
│   ├── clob               // Proof-of-concept CLOB built with InfinityVM
│   └── matching-game      // Simple offchain app built with InfinityVM
├── programs               // Sample zkVM programs
├── proto                  // Proto definitions
├── test                   // e2e and load tests for coprocessor, CLOB, and matching game
```

## Contributing

See the [contributing](./CONTRIBUTING.md) doc for more info on workspace setup, lints, proto, etc.

## Media

InfinityVM Litepaper: https://infinityvm.xyz/infinityvm_litepaper.pdf

[![Twitter](https://img.shields.io/twitter/url/https/twitter.com/infinity_vm.svg?style=social&label=Follow%20%40infinity_vm)](https://twitter.com/infinity_vm)
