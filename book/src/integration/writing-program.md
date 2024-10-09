# Writing a zkVM Program

The InfinityVM coprocessor runs zkVM programs. While the zkVM abstracts is extensible across SP1, Jolt, Risc0 and more - the current implementation just has support for Risc0. Please, let us know if you would like support for another zkVM, its fairly straightforward to add!

If you just want to quickly get your hands dirty, head over the [InfinityVM foundry template](1)

## Anatomy of a program

A zkVM program is (normally) written and rust and compiled down to a RISC-V set of instructions. The executable file is commonly referred to as an [ELF](4).

The logic for a program is:

1. Read input bytes from host
1. Deserialize input bytes
1. Run logic
1. Serialize output bytes
1. Write output bytes to host

Lets take the [CLOB program](5) as an example. For this exercise we will incrementally build out the program

First we read in opaque bytes for each type and deserialize

```rust
fn main() {
  // First we read in 4 bytes from the host and deserialize that to a u32
  let onchain_input_len: u32 = risc0_zkvm::guest::env::read();
  /// We use the u32 as the length, in bytes, for the next expected type
  let mut onchain_input_buf = vec![0; onchain_input_len as usize];
  // We read in exactly  the next `onchain_input_len` number of bytes from the host.
  risc0_zkvm::guest::env::read_slice(&mut onchain_input_buf);
  // In practice, `onchain_input_buf` is always length 0. This input exists as
  // part of the expected api for an offchain job, but the CLOB doesn't have any
  // inputs that get persisted onchain

  // We follow the same steps as above to read in a continuous series of bytes 
  // of a given length from the host
  let offchain_input_len: u32 = env::read();
  let mut offchain_input_buf = vec![0; offchain_input_len as usize];
  env::read_slice(&mut offchain_input_buf);
  // Then we deserialize the bytes to a vector of requests.
  let requests: Vec<Request> = borsh::from_slice(&offchain_input_buf);

  // Following the same pattern as the above, we deserialize the clob state
  let state_len: u32 = env::read();
  let mut state_buf = vec![0; state_len as usize];
  env::read_slice(&mut state_buf);
  let state: ClobState = borsh::from_slice(&state_buf);

  // ..
}
```

Then we run logic

```rust
fn main() {
  // .. reading and deserialization logic

  // Now we run the requests and state the CLOB state transition function
  let clob_program_output = zkvm_stf(requests, state);

  // ..
}
```

Finally, we serialize the output and write it to the host

```rust
fn main() {
  // .. run logic

  /// Serialize the result bytes.
  let abi_encoded = StatefulProgramResult::abi_encode(&clob_program_output);

  /// Write the raw, serialized bytes to the host. Note that this will get posted onchain
  env::commit_slice(&abi_encoded);
}
```

Note the job request is created by the app server bundling requests and the state just prior to the initial request. The abi encoded output contains a hash of the resulting state and is used onchain to verify that the next job request has the correct state input.

## Code organization

Assuming you organize the main function of your zkVM program as above, you will have the business logic encapsulated in a single, pure function. Its recommended to define this function in a separate crate such that the code can be easily reused and unit tested without the restrictions of the zkVM host, which can be prohibitive in using typical rust tooling.

For example, the clob program has a [stf function](stf) defined in a core crate. This function is a wrapper around the [clob engines tick function](stf-tick), which processes a single request at a time. By design, the app server engine uses this same exact [tick function](engine-tick) to process each request.

One thing to keep in mind is that any of the depencies of the zkVM program will need to be compatible with the VM; roughly 70% of major crates are compatible with the vm. However, trouble shooting issues with incompatible deps can be non-trivial and it will force your crate with core logic to not contain deps like [alloy](alloy-features) with the [`full` feature](alloy-full) enabled. Don't hesitate to reach out to the InfinityVM team if you are having any persistent challenges!

## Testing your program

For direct unit tests of your program, you can create a executor and run it against inputs and the program ELF. An example with the CLOB program can be found [here](clob-unit)

## Further reading

- (InfinityVM foundry template)[1]
- (Offchain Jobs section)[2]
- (CLOB example section)[3]

[1]: https://github.com/InfinityVM/infinity-foundry-template
[2]: offchain.md
[3]: clob.md
[4]: https://en.wikipedia.org/wiki/Executable_and_Linkable_Format
[5]: https://github.com/InfinityVM/InfinityVM/blob/main/clob/programs/app/src/clob.rs
[engine-tick]: https://github.com/InfinityVM/InfinityVM/blob/f0d3e956e67d07e68a2670ebbafe6a34839f3df5/clob/node/src/engine.rs#L66
[stf-tick]: https://github.com/InfinityVM/InfinityVM/blob/f0d3e956e67d07e68a2670ebbafe6a34839f3df5/clob/core/src/lib.rs#L282
[tick-unit]: https://github.com/InfinityVM/InfinityVM/blob/f0d3e956e67d07e68a2670ebbafe6a34839f3df5/clob/core/src/lib.rs#L348
[stf]: https://github.com/InfinityVM/InfinityVM/blob/f0d3e956e67d07e68a2670ebbafe6a34839f3df5/clob/core/src/lib.rs#L275
[alloy-full]: https://github.com/alloy-rs/alloy/blob/3f5f1e5de21552ed875ffdc16fb4d5db9d1ba0e8/crates/alloy/Cargo.toml#L76
[alloy-features]: https://docs.rs/crate/alloy/latest/features
[clob-unit]: https://github.com/InfinityVM/InfinityVM/blob/f0d3e956e67d07e68a2670ebbafe6a34839f3df5/clob/programs/src/lib.rs#L120