//! Build script for buidling zkvm programs and embedding them in test contracts.

// use clob_zkvm_utils::sol::{generate_solidity_files, Options};

// // Paths where the generated Solidity files will be written.
// const SOLIDITY_PROGRAM_ID_PATH: &str = "../../contracts/src/ProgramID.sol";
// const SOLIDITY_DEPLOY_SCRIPT_PATH: &str = "../../contracts/script/Deployer.s.sol";

fn main() {
    // let guests = risc0_build::embed_methods();

    // // Generate Solidity source files for use with Forge.
    // let solidity_opts = Options::default()
    //     .with_program_id_sol_path(SOLIDITY_PROGRAM_ID_PATH)
    //     .with_deploy_script_path(SOLIDITY_DEPLOY_SCRIPT_PATH);

    // generate_solidity_files(guests.as_slice(), &solidity_opts).unwrap();

    risc0_build::embed_methods();
}
