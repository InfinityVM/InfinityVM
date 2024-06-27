//! Better then your ZKVM.

/// The error
#[derive(Debug)]
pub enum Error {}

/// A ZKVM
pub trait ZKVM {
    /// Check if the verifying key can be derived from program elf.
    fn is_correct_verifying_key(program_elf: Vec<u8>, program_verifying_key: Vec<u8>) -> bool;

    /// Execute the program and return the raw output.
    ///
    /// This does _not_ check that the verifying key is correct.
    fn execute(program_elf: Vec<u8>, program_input: Vec<u8>) -> Result<Vec<u8>, Error>;

    // methods for pessimists
    // fn prove()
    // fn verify()
}
