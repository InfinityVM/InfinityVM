//! ZKVM trait and implementations. The trait should abstract over any complexities to specific VMs.

use risc0_binfmt::compute_image_id;
use risc0_zkvm::{Executor as Risc0Executor, ExecutorEnv, LocalProver};
use sp1_sdk::{HashableKey, ProverClient};
use thiserror::Error;

/// The error
#[derive(Error, Debug)]
pub enum Error {
    /// Error from the Risc0
    #[error("Risc0 error: {source}")]
    Risc0 {
        /// The underlying error from Risc0
        source: anyhow::Error,
    },

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
    /// Derive the verifying key from an elf
    fn derive_verifying_key(&self, program_elf: &[u8]) -> Result<Vec<u8>, Error>;

    /// Execute the program and return the raw output.
    ///
    /// This does _not_ check that the verifying key is correct.
    fn execute(
        &self,
        program_elf: &[u8],
        onchain_input: &[u8],
        offchain_input: &[u8],
        max_cycles: u64,
    ) -> Result<Vec<u8>, Error>;

    /// Check if the verifying key can be derived from program elf.
    fn is_correct_verifying_key(
        &self,
        program_elf: &[u8],
        program_verifying_key: &[u8],
    ) -> Result<bool, Error> {
        let derived_verifying_key = self.derive_verifying_key(program_elf)?;

        Ok(derived_verifying_key == program_verifying_key)
    }

    // methods for pessimists
    // fn prove()
    // fn verify()
}

/// Risc0 impl of [Zkvm].
#[derive(Debug)]
pub struct Risc0;

impl Zkvm for Risc0 {
    fn derive_verifying_key(&self, program_elf: &[u8]) -> Result<Vec<u8>, Error> {
        let image_id = compute_image_id(program_elf)
            .map_err(|source| Error::Risc0 { source })?
            .as_bytes()
            .to_vec();

        Ok(image_id)
    }

    fn execute(
        &self,
        program_elf: &[u8],
        onchain_input: &[u8],
        offchain_input: &[u8],
        max_cycles: u64,
    ) -> Result<Vec<u8>, Error> {
        let onchain_input_len = onchain_input.len() as u32;
        let offchain_input_len = offchain_input.len() as u32;
        let env = ExecutorEnv::builder()
            .session_limit(Some(max_cycles))
            .write(&onchain_input_len)
            .map_err(|source| Error::Risc0 { source })?
            .write_slice(onchain_input)
            .write(&offchain_input_len)
            .map_err(|source| Error::Risc0 { source })?
            .write_slice(offchain_input)
            .build()
            .map_err(|source| Error::Risc0 { source })?;

        let prover = LocalProver::new("locals only");
        let prove_info =
            prover.execute(env, program_elf).map_err(|source| Error::Risc0 { source })?;

        Ok(prove_info.journal.bytes)
    }
}

/// Sp1 impl of [Zkvm], includes the prover client.
#[derive(Debug)]
pub struct Sp1;

impl Zkvm for Sp1 {
    fn derive_verifying_key(&self, program_elf: &[u8]) -> Result<Vec<u8>, Error> {
        let (_, vk) = ProverClient::new().setup(program_elf);
        Ok(vk.hash_bytes().to_vec())
    }

    fn execute(
        &self,
        program_elf: &[u8],
        onchain_input: &[u8],
        offchain_input: &[u8],
        max_cycles: u64,
    ) -> Result<Vec<u8>, Error> {
        use sp1_sdk::SP1Stdin;
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
    use crate::{Risc0, Sp1, Zkvm};
    use alloy::sol_types::SolValue;
    use mock_consumer::{mock_contract_input_addr, mock_raw_output, MOCK_CONSUMER_MAX_CYCLES};
    use mock_consumer_sp1::SP1_MOCK_CONSUMER_GUEST_ELF;

    const MOCK_CONSUMER_ELF_PATH: &str =
        "../../target/riscv-guest/riscv32im-risc0-zkvm-elf/release/mock-consumer-risc0-guest";

    #[test]
    fn risc0_execute_can_correctly_execute_program() {
        let elf = std::fs::read(MOCK_CONSUMER_ELF_PATH).unwrap();

        let input = mock_contract_input_addr();
        let raw_input = input.abi_encode();

        let raw_result = &Risc0.execute(&elf, &raw_input, &[], MOCK_CONSUMER_MAX_CYCLES).unwrap();

        assert_eq!(*raw_result, mock_raw_output());
    }

    #[test]
    fn risc0_is_correct_verifying_key() {
        let elf = std::fs::read(MOCK_CONSUMER_ELF_PATH).unwrap();
        let mut image_id = risc0_binfmt::compute_image_id(&elf).unwrap().as_bytes().to_vec();

        let correct = &Risc0.is_correct_verifying_key(&elf, &image_id).unwrap();
        assert!(correct);

        image_id.pop();
        image_id.push(255);

        let correct = &Risc0.is_correct_verifying_key(&elf, &image_id).unwrap();

        assert!(!correct);
    }
    #[test]
    fn sp1_execute_can_correctly_execute_program() {
        let input = mock_contract_input_addr();
        let raw_input = input.abi_encode();

        let raw_result = &Sp1
            .execute(SP1_MOCK_CONSUMER_GUEST_ELF, &raw_input, &[], MOCK_CONSUMER_MAX_CYCLES)
            .unwrap();

        assert_eq!(*raw_result, mock_raw_output());
    }
}
