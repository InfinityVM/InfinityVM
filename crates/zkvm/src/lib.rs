//! ZKVM trait and implementations. The trait should abstract over any complexities to specific VMs.

use sp1_core_executor::{Executor, SP1Context};
use sp1_sdk::{HashableKey, Prover, ProverClient, SP1Stdin};
use thiserror::Error;

/// The error
#[derive(Error, Debug)]
pub enum Error {
    /// Error from the Sp1
    #[error("Sp1 error: {source}")]
    Sp1 {
        /// The underlying error from the Sp1
        source: anyhow::Error,
    },

    /// Error exceeded cycle limit
    #[error("Cycle limit exceeded")]
    CycleLimitExceeded,
}

/// Something that can execute programs and generate ZK proofs for them.
pub trait Zkvm {
    /// Derive the program ID from an elf
    fn derive_program_id(&self, program_elf: &[u8]) -> Result<Vec<u8>, Error>;

    /// Execute the program and return the raw output.
    ///
    /// This does _not_ check that the program ID is correct.
    fn execute(
        &self,
        program_elf: &[u8],
        onchain_input: &[u8],
        offchain_input: &[u8],
        max_cycles: u64,
    ) -> Result<Vec<u8>, Error>;

    /// Check if the program ID can be derived from program elf.
    fn is_correct_program_id(&self, program_elf: &[u8], program_id: &[u8]) -> Result<bool, Error> {
        let derived_program_id = self.derive_program_id(program_elf)?;

        Ok(derived_program_id == program_id)
    }

    // methods for pessimists
    // fn prove()
    // fn verify()
}

/// Sp1 impl of [Zkvm], includes the prover client.
#[derive(Debug, Clone)]
pub struct Sp1;

impl Zkvm for Sp1 {
    fn derive_program_id(&self, program_elf: &[u8]) -> Result<Vec<u8>, Error> {
        let (_, program_id) = ProverClient::builder().cpu().build().setup(program_elf);

        let program_id_hash = program_id.hash_bytes().to_vec();
        Ok(program_id_hash)
    }

    fn execute(
        &self,
        program_elf: &[u8],
        onchain_input: &[u8],
        offchain_input: &[u8],
        max_cycles: u64,
    ) -> Result<Vec<u8>, Error> {
        // For now we mostly copy the logic from the prover execute method
        // https://github.com/succinctlabs/sp1/blob/d3f451919f5a00c21e44ece80f576860a09a1da9/crates/prover/src/lib.rs#L292
        let mut stdin = SP1Stdin::new();
        stdin.write_slice(onchain_input);
        stdin.write_slice(offchain_input);

        let core_opts = Default::default();
        // We can change this once we impl https://github.com/InfinityVM/InfinityVM/issues/338
        let mut runtime =
            Executor::with_context_and_elf(core_opts, SP1Context::default(), program_elf);
        runtime.max_cycles = Some(max_cycles);
        runtime.write_vecs(&stdin.buffer);
        runtime.run_fast().expect("todo");

        Ok(runtime.state.public_values_stream)
    }
}

#[cfg(test)]
mod test {
    use crate::{Sp1, Zkvm};
    use alloy::sol_types::SolValue;
    use ivm_mock_consumer::{mock_contract_input_addr, mock_raw_output, MOCK_CONSUMER_MAX_CYCLES};
    use mock_consumer_programs::MOCK_CONSUMER_ELF;

    #[test]
    fn sp1_execute_can_correctly_execute_program() {
        let input = mock_contract_input_addr();
        let raw_input = input.abi_encode();

        let raw_result =
            &Sp1.execute(MOCK_CONSUMER_ELF, &raw_input, &[], MOCK_CONSUMER_MAX_CYCLES).unwrap();

        assert_eq!(*raw_result, mock_raw_output());
    }

    #[test]
    fn sp1_is_correct_program_id() {
        let program_id_bytes = mock_consumer_programs::MOCK_CONSUMER_PROGRAM_ID.to_vec();

        let correct = &Sp1.is_correct_program_id(MOCK_CONSUMER_ELF, &program_id_bytes).unwrap();
        assert!(correct);

        let mut modified_program_id = program_id_bytes;
        modified_program_id.pop();
        modified_program_id.push(255);

        let correct = &Sp1.is_correct_program_id(MOCK_CONSUMER_ELF, &modified_program_id).unwrap();
        assert!(!correct);
    }
}
