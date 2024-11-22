# Writing a zkVM Program

The InfinityVM coprocessor runs zkVM programs. While the zkVM abstraction is extensible across different VMs, the current implementation has support for SP1. Please let us know if you would like support for another zkVM, it's fairly straightforward to add!

If you just want to quickly get your hands dirty, head over to the [InfinityVM foundry template](https://github.com/InfinityVM/infinityVM-foundry-template). You can fork the repo and we have instructions on how to write a program in the `README`.

## Structure of a program

A zkVM program is written in Rust and compiles down to RISC-V. The executable file is commonly referred to as an [ELF](https://en.wikipedia.org/wiki/Executable_and_Linkable_Format).

The logic for a program is:

1. Read input bytes (e.g. `sp1_zkvm::io::read_vec()`)
2. Deserialize input bytes (e.g. `U256::abi_decode()`) 
3. Run logic (e.g. `number.root(2)`)
4. Serialize output bytes (e.g. `SolType::abi_encode()`)
5. Write output bytes (e.g. `sp1_zkvm::io::commit_slice()`)

Lets take a [square root program](https://github.com/InfinityVM/infinityVM-foundry-template/blob/main/programs/square-root/src/main.rs) as an example. This program takes in a number as input and returns the square root of the number as output. For this exercise we will incrementally build out the program.

First, we read in onchain input and parse:

```rust,ignore
#![no_main]
sp1_zkvm::entrypoint!(main);

fn main() {

    // This application only uses onchain input. We read the onchain input here.
    let onchain_input = sp1_zkvm::io::read_vec();
    // Decode and parse the input
    let number = <U256>::abi_decode(&onchain_input, true).unwrap();

    // ..
}
```

Next, we run logic (computing the square root) on the input `number`:

```rust,ignore
#![no_main]
sp1_zkvm::entrypoint!(main);

fn main() {
    // .. reading and parsing logic

    // Calculate square root
    let square_root = number.root(2);

    // ..
}
```

Finally, we serialize the output and write it:

```rust,ignore
#![no_main]
sp1_zkvm::entrypoint!(main);

sol! {
    struct NumberWithSquareRoot {
        uint256 number;
        uint256 square_root;
    }
}

fn main() {
    // .. reading and parsing logic

    // .. run logic

    // Commit the output that will be received by the application contract.
    // Output is encoded using Solidity ABI for easy decoding in the app contract.
    let number_with_square_root = NumberWithSquareRoot { number, square_root };

    sp1_zkvm::io::commit_slice(
        <NumberWithSquareRoot as SolType>::abi_encode(&number_with_square_root).as_slice(),
    );

}
```

## Code organization

Assuming you organize the main function of your zkVM program as above, you can have all your logic in a single pure function. If your logic is more complex, you can also write your code in a separate crate such that the code can be easily reused and unit tested without the restrictions of the zkVM.

**Note:** Any of the dependencies you use in your zkVM program need to be compatible with the zkVM; roughly 70% of major crates are compatible. A common issue is the [alloy](https://docs.rs/crate/alloy/latest/features) crate, which works with most of the [features disabled](https://github.com/InfinityVM/InfinityVM/blob/main/Cargo.toml), but breaks builds with the [`full` feature](https://github.com/alloy-rs/alloy/blob/main/crates/alloy/Cargo.toml) enabled. Don't hesitate to reach out to the InfinityVM team if you face any challenges with this!

## Testing your program

If you're using the [InfinityVM foundry template](https://github.com/InfinityVM/infinityVM-foundry-template), you can test and debug your zkVM program itself by following the example [here](https://github.com/InfinityVM/infinityVM-foundry-template/blob/main/programs/src/lib.rs) (you can run this using `cargo test`). You can add `dbg!` statements anywhere to help while debugging.

If you're not using the InfinityVM foundry template, you can write unit tests by creating an executor and running the executor with your inputs and zkVM program ELF. An example with the matching game program can be found [here](https://github.com/InfinityVM/InfinityVM/blob/main/examples/matching-game/programs/src/lib.rs) (More info on this in the [Offchain App (Simple): Matching Game](../apps/matching-game.md) section).

For integration tests, we recommend reading the [Using your zkVM Program](./using-program.md) section.

The InfinityVM team is working on a growing set of [SDK crates](https://github.com/InfinityVM/InfinityVM/tree/main/crates/sdk) to make writing programs and tests easier.
