# Building with InfinityVM

 InfinityVM enables developers to use expressive offchain compute alongside the EVM to create new types of applications. This section discusses key concepts for building an app with InfinityVM.

## Overview

The InfinityVM coprocessor allows developers to run a zkVM program with any set of inputs and then use the results onchain. A zkVM program is written in a language that compiles down to RISC-V (most commonly Rust). It accepts any inputs, runs some compute using these inputs, and then returns the result.

The high-level flow looks like this:

1. An app submits a zkVM program to the coprocessor. The coprocessor returns a program ID (unique identifier for program).
1. An app contract or an offchain user/server requests a `job` from the coprocessor. Each job request contains the program ID and inputs to the program.
1. The coprocessor executes this program with the given inputs in a RISC-V interpreter.
1. The coprocessor submits the result (output of program) back to the contract.
1. The app contract can simply use the result from the coprocessor in any of their app logic. A callback function is called on the app contract, which can then execute arbitrary logic.

![coprocessor flow](../assets/coprocessor-overview.png)

Importantly, InfinityVM guarantees that the canonical execution chain only contains valid job results.

### Writing a zkVM Program

Read more in the [Writing a Program](./writing-program.md) section.

### Using a zkVM Program

A zkVM program can be run in InfinityVM using two types of job requests: **onchain** and **offchain**. Read more in the [Using a Program](./using-program.md) section.

### Examples

We have walked through three examples of building apps with InfinityVM:

- [<u>Onchain App: Square Root</u>](../apps/square-root.md)
- [<u>Offchain App (Simple): Matching Game</u>](../apps/matching-game.md)
- [<u>Offchain App (Advanced): CLOB</u>](../apps/clob.md)
