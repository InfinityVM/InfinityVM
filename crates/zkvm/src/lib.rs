//! ZKVM trait and implementations. The trait should abstract over any complexities to specific VMs.

use sp1_sdk::{HashableKey, ProverClient, SP1Stdin};
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
        let (_, program_id) = ProverClient::new().setup(program_elf);
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
            .map_err(|e| Error::Sp1 { source: e })?;

        Ok(output.to_vec())
    }
}

#[cfg(test)]
mod test {
    use crate::{Sp1, Zkvm};
    use alloy::sol_types::SolValue;
    use mock_consumer::{mock_contract_input_addr, mock_raw_output, MOCK_CONSUMER_MAX_CYCLES};
    use mock_consumer_programs::MOCK_CONSUMER_ELF;
    use sp1_sdk::HashableKey;

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
        let (_, program_id) = sp1_sdk::ProverClient::new().setup(MOCK_CONSUMER_ELF);
        let mut program_id_bytes = program_id.hash_bytes().to_vec();

        let correct = &Sp1.is_correct_program_id(MOCK_CONSUMER_ELF, &program_id_bytes).unwrap();
        assert!(correct);

        program_id_bytes.pop();
        program_id_bytes.push(255);

        let correct = &Sp1.is_correct_program_id(MOCK_CONSUMER_ELF, &program_id_bytes).unwrap();
        assert!(!correct);
    }
}
