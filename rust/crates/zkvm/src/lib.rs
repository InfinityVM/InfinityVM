//! Better then your ZKVM.

use risc0_binfmt::compute_image_id;
use risc0_zkvm::{Executor, ExecutorEnv, LocalProver};
use thiserror::Error;

/// The error
#[derive(Error, Debug)]
pub enum Error {
    /// Error from the Risc0 sdk
    #[error(transparent)]
    Risc0(#[from] anyhow::Error),
}

/// A ZKVM
pub trait Zkvm {
    /// Check if the verifying key can be derived from program elf.
    fn is_correct_verifying_key(
        program_elf: &[u8],
        program_verifying_key: &[u8],
    ) -> Result<bool, Error>;

    /// Execute the program and return the raw output.
    ///
    /// This does _not_ check that the verifying key is correct.
    fn execute(program_elf: &[u8], raw_input: &[u8], max_cycles: u64) -> Result<Vec<u8>, Error>;

    // methods for pessimists
    // fn prove()
    // fn verify()
}

pub struct Risc0;

impl Zkvm for Risc0 {
    fn is_correct_verifying_key(
        program_elf: &[u8],
        program_verifying_key: &[u8],
    ) -> Result<bool, Error> {
        let image_id = compute_image_id(program_elf)?;
        let is_correct = image_id.as_bytes() == program_verifying_key;

        Ok(is_correct)
    }

    fn execute(program_elf: &[u8], raw_input: &[u8], max_cycles: u64) -> Result<Vec<u8>, Error> {
        let env = ExecutorEnv::builder()
            .session_limit(Some(max_cycles))
            .write_slice(raw_input)
            .build()?;

        let prover = LocalProver::new("locals only");

        let prove_info = prover.execute(env, program_elf)?;
        // let receipt = prove_info.receipt;

        // TODO(zeke): verify the raw bytes are whats wrong here
        Ok(prove_info.journal.bytes)
    }
}

#[cfg(test)]
mod test {
    fn risc0_execute_works() {
        // take a real program and make sure
        // - we can serialize bytes
        // - run execute
        // - deserialize result
    }
}
