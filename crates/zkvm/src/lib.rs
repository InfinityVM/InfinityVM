//! ZKVM trait and implementations. The trait should abstract over any complexities to specific VMs.

use sp1_sdk::{HashableKey, ProverClient, SP1Stdin};
use thiserror::Error;

/// The error
#[derive(Error, Debug, Clone)]
pub enum Error {
    /// Error from the Sp1
    #[error("Sp1 error: {msg}")]
    Sp1 {
        /// The error message
        msg: String,
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
        let client = ProverClient::new();
        // ProverClient::setup can panic, but we'll let it propagate since it indicates
        // a fundamental issue with the ELF that should fail the verification
        let (_, program_id) = client.setup(program_elf);
        Ok(program_id.hash_bytes().to_vec())
    }

    fn execute(
        &self,
        program_elf: &[u8],
        onchain_input: &[u8],
        offchain_input: &[u8],
        max_cycles: u64,
    ) -> Result<Vec<u8>, Error> {
        let mut stdin = SP1Stdin::new();
        stdin.write_slice(onchain_input);
        stdin.write_slice(offchain_input);

        let client = ProverClient::new();
        let (output, _) = client
            .execute(program_elf, stdin)
            .max_cycles(max_cycles)
            .run()
            .map_err(|e| Error::Sp1 { msg: e.to_string() })?;

        Ok(output.to_vec())
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

        // Test with correct program ID
        match Sp1.is_correct_program_id(MOCK_CONSUMER_ELF, &program_id_bytes) {
            Ok(correct) => assert!(correct, "Program ID should match"),
            Err(e) => panic!("Program ID verification failed: {}", e),
        }

        // Test with modified program ID
        let mut modified_program_id = program_id_bytes;
        modified_program_id.pop();
        modified_program_id.push(255);

        match Sp1.is_correct_program_id(MOCK_CONSUMER_ELF, &modified_program_id) {
            Ok(correct) => assert!(!correct, "Modified program ID should not match"),
            Err(e) => panic!("Program ID verification failed: {}", e),
        }
    }
}
