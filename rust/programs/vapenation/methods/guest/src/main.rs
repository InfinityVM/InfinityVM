//! NOTE(zeke): The stuff in here will be compiled down to riscv32im byte code.
//!
//! Anything that you want to be accessible in the host (i.e. stuff that
//! compiles down your local arch), must be defined in another crate and then
//! imported
//!
//! For this example, we are putting shared resources in a `core` crate.
use risc0_zkvm::guest::env;
use core::{VapeNationMetadata, compute_nation_id};

fn main() {
    // read in data as bytes
    let mut raw_input = [0u8; 8];
    env::read_slice(&mut raw_input);
    // deserialize the bytes to a u64
    let input = u64::from_be_bytes(raw_input);
    
    // Note that alternatively we could have done, but this would mean
    // serializing/deserializing
    // let input: u64 = env::read();

    let phrase = (0..input)
        .map(|_| "NeverForget420".to_string())
        .collect::<Vec<_>>()
        .join(" ");

    // We can use functions defined in other crates
   let nation_id = compute_nation_id(input);

    let points = nation_id / 69;


    let output = VapeNationMetadata {
        nation_id,
        phrase,
        points,
    };

    // write public output to the journal
    // TODO(zeke): commit output just as lslice
    env::commit(&output);
}
