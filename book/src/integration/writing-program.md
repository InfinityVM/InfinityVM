# Writing a zkVM Program

The InfinityVM coprocessor runs zkVM programs. While the zkVM abstraction is extensible across SP1, Jolt, Risc0 and more - the current implementation just has support for Risc0. Please, let us know if you would like support for another zkVM, its fairly straightforward to add!

If you just want to quickly get your hands dirty, head over the [InfinityVM foundry template](https://github.com/InfinityVM/infinity-foundry-template)

## Anatomy of a program

A zkVM program is (normally) written and rust and compiled down to a RISC-V set of instructions. The executable file is commonly referred to as an [ELF](https://en.wikipedia.org/wiki/Executable_and_Linkable_Format).

The logic for a program is:

1. Read input bytes from host
1. Deserialize input bytes
1. Run logic
1. Serialize output bytes
1. Write output bytes to host

Lets take the [square program](https://github.com/InfinityVM/infinity-foundry-template/blob/main/programs/app/src/square-root.rs) as an example. For this exercise we will incrementally build out the program.

First we read in opaque bytes for each type and deserialize

```rust,ignore
fn main() {
  // Create an empty buffer
  let mut input_bytes = Vec::<u8>::new();
  // Read in all the bytes from the host to buffer
  risc0_zkvm::guest::env::stdin().read_to_end(&mut input_bytes).unwrap();

  // Deserialize the buffer into the expected type, U256
  let number = <U256>::abi_decode(&input_bytes, true).unwrap();

  // ..
}
```

Then we run logic

```rust,ignore
fn main() {
  // .. reading and deserialization logic

  // Run the business logic
  let square_root = number.root(2);

  // ..
}
```

Finally, we serialize the output and write it to the host

```rust,ignore
type NumberWithSquareRoot = sol! {
    tuple(uint256,uint256)
};

fn main() {
  // .. run logic

  /// Serialize the result bytes.
  let abi_encoded = NumberWithSquareRoot::abi_encode(&(number, square_root)

  /// Write the raw, serialized bytes to the host. Note that this will get posted onchain
  risc0_zkvm::guest::env::commit_slice(&abi_encoded);
}
```

## Code organization

Assuming you organize the main function of your zkVM program as above, you can have your business logic encapsulated in a pure function. If your logic is non-trivial, its recommended to define this function in a separate crate such that the code can be easily reused and unit tested without the restrictions of the zkVM host, which can be prohibitive in using typical rust tooling.

For example, the clob program has a [stf function](https://github.com/InfinityVM/InfinityVM/blob/f0d3e956e67d07e68a2670ebbafe6a34839f3df5/clob/core/src/lib.rs#L275) defined in a "core" crate. This function is a wrapper around the [clob engine's tick function](https://github.com/InfinityVM/InfinityVM/blob/f0d3e956e67d07e68a2670ebbafe6a34839f3df5/clob/core/src/lib.rs#L282), which processes a single request at a time. By design, the app server engine uses this same exact [tick function](https://github.com/InfinityVM/InfinityVM/blob/f0d3e956e67d07e68a2670ebbafe6a34839f3df5/clob/node/src/engine.rs) to process each request.

One thing to keep in mind is that any of the dependencies of the zkVM program will need to be compatible with the VM; roughly 70% of major crates are compatible with the VM. However, trouble shooting issues with incompatible deps can be non-trivial and it will force your crate with core logic to not contain incompatible deps. A common source of pain is the [alloy](https://docs.rs/crate/alloy/latest/features) crate, which works with most of the [features disabled](https://github.com/InfinityVM/InfinityVM/blob/f0d3e956e67d07e68a2670ebbafe6a34839f3df5/Cargo.toml#L118), but breaks builds with the [`full` feature](https://github.com/alloy-rs/alloy/blob/3f5f1e5de21552ed875ffdc16fb4d5db9d1ba0e8/crates/alloy/Cargo.toml#L76) enabled. Don't hesitate to reach out to the InfinityVM team if you are having any persistent challenges!

## Testing your program

For direct unit tests of your program, you can create an executor and run it against inputs and the program ELF. An example with the CLOB program can be found [here](https://github.com/InfinityVM/InfinityVM/blob/f0d3e956e67d07e68a2670ebbafe6a34839f3df5/clob/programs/src/lib.rs#L120).

For integration tests with the EVM for onchain requests and stateless offchain requests, you can use the [foundry template](https://github.com/InfinityVM/infinity-foundry-template).

For integration tests for stateful requests from an app server, you will need to build out a custom test harness. You can find an example test harness with the coprocessor and clob [here](https://github.com/InfinityVM/InfinityVM/blob/main/test/e2e/src/lib.rs). 

The InfinityVM team is working on a growing set of [SDK crates](https://github.com/InfinityVM/InfinityVM/tree/main/crates/sdk) to make writing programs and tests easier. The SDK is in very early stages and dog food'ed with the CLOB app server PoC.
