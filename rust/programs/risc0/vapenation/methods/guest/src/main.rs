//! This is a SUPER SERIOUS ZKVM PROGRAM. It is extremely critical and if you
//! have to ask what it is you probably should reconsider your career.
//! JK! This is just a silly program to demonstrate how to setup a risc0
//! program that can be used with our executor.
//!
//! The stuff in here will be compiled down to riscv32im byte code.
//!
//! Anything that you want to be accessible in the host (i.e. stuff that
//! compiles down to your local arch), must be defined in another crate and then
//! imported,
//!
//! For this example, we are putting shared resources in a `core` crate.
use alloy_sol_types::SolType;
use risc0_zkvm::guest::env;
use vapenation_core::{compute_nation_id, VapeNationArg, VapeNationMetadata};
use std::io::Read;

fn main() {
    // read in data as bytes
    let mut raw_input = vec![];
    env::stdin().read_to_end(&mut raw_input).unwrap();

    // deserialize the bytes to a u64 using rlp encoding
    let input = VapeNationArg::abi_decode(&raw_input, false).unwrap();

    // Note that alternatively we could have done, but this would mean
    // serializing/deserializing implicitly with the risc0 serialization.
    // let input: u64 = env::read();

    let phrase = (0..input).map(|_| "NeverForget420".to_string()).collect::<Vec<_>>().join(" ");

    // We can use functions defined in other crates
    let nation_id = compute_nation_id(input);

    let points = nation_id / 69;

    let output = alloy_rlp::encode(VapeNationMetadata { nation_id, phrase, points });

    env::commit_slice(&output);
}
