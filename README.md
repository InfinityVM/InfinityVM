![infinityvm banner](./book/src/assets/infinityvm-banner.png)

## A new blockchain architecture with native offchain compute

InfinityVM enables developers to use expressive offchain compute alongside the EVM to create new types of applications.

## Docs

Consult the [InfinityVM Book](./book) for detailed docs on:

- How to build an app with InfinityVM
- InfinityVM contracts
- InfinityVM coprocessor node
- Infinity L1 architecture

## Contributing

See the [contributing](./CONTRIBUTING.md) doc for instructions on how to setup the workspace and run tests.

## Directory Structure

The following are some of the more important directories in the InfinityVM repository:

```shell
.
├── contracts              // Onchain business logic
├── crates                 // InfinityVM coprocessor
│   ├── coprocessor-node   // Core logic of coprocessor
│   ├── db                 // Database for coprocessor node
│   ├── zkvm-executor      // zkVM interface used by coprocessor
│   └── zkvm               // zkVM trait and implementations      
├── examples               // Example apps built with InfinityVM
│   ├── clob               // Proof-of-concept CLOB built with InfinityVM
│   └── matching-game      // Simple offchain app built with InfinityVM
├── programs               // Sample zkVM programs
├── proto                  // Proto definitions
├── test                   // e2e and load tests for coprocessor and CLOB
```

## Media

InfinityVM Litepaper: https://infinityvm.xyz/infinityvm_litepaper.pdf

[![Twitter](https://img.shields.io/twitter/url/https/twitter.com/infinity_vm.svg?style=social&label=Follow%20%40infinity_vm)](https://twitter.com/infinity_vm)
