// To run execute:
// RUST_LOG=info cargo run --release -- --execute
use alloy::primitives::Address;
use clap::Parser;
use mock_consumer_sp1::SP1_MOCK_CONSUMER_GUEST_ELF;
use sp1_sdk::{ProverClient, SP1Stdin};

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(long)]
    execute: bool,

    #[clap(long)]
    prove: bool,
}
// This is a script to test SP1 mock consumer execution
// proving still WIP
fn main() {
    // Setup the logger.
    sp1_sdk::utils::setup_logger();

    // Parse the command line arguments.
    let args = Args::parse();

    if args.execute == args.prove {
        eprintln!("Error: You must specify either --execute or --prove");
        std::process::exit(1);
    }

    // Setup the prover client.
    let client = ProverClient::new();

    // Create a test address and pad it to 32 bytes
    let address = Address::repeat_byte(69);
    let mut input = vec![0u8; 32];
    input[12..32].copy_from_slice(address.as_ref());

    // Setup the inputs.
    let mut stdin = SP1Stdin::new();
    stdin.write_slice(&input);

    if args.execute {
        // Execute the program
        let (output, report) = client.execute(SP1_MOCK_CONSUMER_GUEST_ELF, stdin).run().unwrap();
        println!("Program executed successfully.");
        println!("Output: {:?}", output);
        println!("Number of cycles: {}", report.total_instruction_count());
    } else {
        // Setup the program for proving.
        let (pk, vk) = client.setup(SP1_MOCK_CONSUMER_GUEST_ELF);

        // Generate the proof
        let proof = client.prove(&pk, stdin).run().expect("failed to generate proof");

        println!("Successfully generated proof!");

        // Verify the proof.
        client.verify(&proof, &vk).expect("failed to verify proof");
        println!("Successfully verified proof!");
    }
}
