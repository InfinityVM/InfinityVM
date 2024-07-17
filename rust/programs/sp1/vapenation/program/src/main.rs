//! Run at your own sp1
#![no_main]
sp1_zkvm::entrypoint!(main);

use alloy_sol_types::SolType;
use vapenation_core::{compute_nation_id, VapeNationArg, VapeNationMetadata};

fn main() {
    // read in data as bytes
    let raw_input = sp1_zkvm::io::read_vec();

    // deserialize the bytes to a u64 using rlp encoding
    let input = VapeNationArg::abi_decode(&raw_input, false).unwrap();

    let phrase = (0..input).map(|_| "NeverForget420".to_string()).collect::<Vec<_>>().join(" ");

    // We can use functions defined in other crates
    let nation_id = compute_nation_id(input);

    let points = nation_id / 69;

    let output = alloy_rlp::encode(VapeNationMetadata { nation_id, phrase, points });

    sp1_zkvm::io::commit_slice(&output);
}