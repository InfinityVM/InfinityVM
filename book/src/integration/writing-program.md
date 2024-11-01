# Writing a zkVM Program

The InfinityVM coprocessor runs zkVM programs. While the zkVM abstraction is extensible across SP1, Risc0, Jolt, and more - the current implementation just has support for Risc0. Please let us know if you would like support for another zkVM, it's fairly straightforward to add!

If you just want to quickly get your hands dirty, head over to the [InfinityVM foundry template](https://github.com/InfinityVM/infinityVM-foundry-template). You can fork the repo and we have instructions on how to write a program in the `README`.

## Structure of a program

A zkVM program is written in a language that compiles down to RISC-V. The executable file is commonly referred to as an [ELF](https://en.wikipedia.org/wiki/Executable_and_Linkable_Format).

The logic for a program is:

1. Read input bytes
1. Deserialize input bytes
1. Run logic
1. Serialize output bytes
1. Write output bytes

Lets take a [square root program](https://github.com/InfinityVM/infinityVM-foundry-template/blob/main/programs/app/src/square_root.rs) as an example. This program takes in a number as input and returns the square root of the number as output. For this exercise we will incrementally build out the program.

First, we read in opaque bytes for the inputs and deserialize:

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

Next, we run logic (computing the square root) on the input `number`:

```rust,ignore
fn main() {
  // .. reading and deserialization logic

  // Run the business logic
  let square_root = number.root(2);

  // ..
}
```

Finally, we serialize the output and write it:

```rust,ignore
type NumberWithSquareRoot = sol! {
    tuple(uint256,uint256)
};

fn main() {
  // .. reading and deserialization logic

  // .. run logic

  // Serialize the result bytes
  let abi_encoded_output = NumberWithSquareRoot::abi_encode(&(number, square_root));

  // Write the raw, serialized bytes to the host. This will get posted onchain
  risc0_zkvm::guest::env::commit_slice(&abi_encoded_output);
}
```

## Code organization

Assuming you organize the main function of your zkVM program as above, you can have all your logic in a single pure function. If your logic is more complex, you can also write your code in a separate crate such that the code can be easily reused and unit tested without the restrictions of the zkVM.

**Note:** Any of the dependencies you use in your zkVM program need to be compatible with the zkVM; roughly 70% of major crates are compatible. A common issue is the [alloy](https://docs.rs/crate/alloy/latest/features) crate, which works with most of the [features disabled](https://github.com/InfinityVM/InfinityVM/blob/main/Cargo.toml#L118), but breaks builds with the [`full` feature](https://github.com/alloy-rs/alloy/blob/main/crates/alloy/Cargo.toml#L76) enabled. Don't hesitate to reach out to the InfinityVM team if you face any challenges with this!

## Testing your program

If you're using the [InfinityVM foundry template](https://github.com/InfinityVM/infinityVM-foundry-template), you can test and debug your zkVM program itself by following the example [here](https://github.com/InfinityVM/infinityVM-foundry-template/blob/main/programs/src/lib.rs) (you can run this using `cargo test`). You can add `dbg!` statements anywhere to help while debugging.

If you're not using the InfinityVM foundry template, you can write unit tests by creating an executor and running the executor with your inputs and zkVM program ELF. An example with the matching game program can be found [here](https://github.com/InfinityVM/InfinityVM/blob/main/examples/matching-game/programs/src/lib.rs#L48) (More info on this in the [Offchain App (Simple): Matching Game](../apps/matching-game.md) section).

For integration tests, we recommend reading the [Using your zkVM Program](./using-program.md) section.

The InfinityVM team is working on a growing set of [SDK crates](https://github.com/InfinityVM/InfinityVM/tree/main/crates/sdk) to make writing programs and tests easier.
