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
    use mock_consumer_sp1::MOCK_CONSUMER_SP1_GUEST_ELF;
    use sp1_sdk::HashableKey;
    const HIGH_CYCLE_LIMIT: u64 = 100_000_000;
    use borsh::BorshSerialize;

    #[test]
    fn risc0_execute_can_correctly_execute_program() {
        const MOCK_CONSUMER_RISC0_GUEST_ELF: &str =
            "../../target/riscv-guest/riscv32im-risc0-zkvm-elf/release/mock-consumer-risc0-guest";
        let elf = std::fs::read(MOCK_CONSUMER_RISC0_GUEST_ELF).unwrap();

        let input = mock_contract_input_addr();
        let raw_input = input.abi_encode();

        let raw_result = &Risc0.execute(&elf, &raw_input, &[], MOCK_CONSUMER_MAX_CYCLES).unwrap();

        assert_eq!(*raw_result, mock_raw_output());
    }

    #[test]
    fn risc0_is_correct_verifying_key() {
        let elf = MOCK_CONSUMER_SP1_GUEST_ELF;
        let mut image_id = risc0_binfmt::compute_image_id(elf).unwrap().as_bytes().to_vec();

        let correct = &Risc0.is_correct_verifying_key(elf, &image_id).unwrap();
        assert!(correct);

        image_id.pop();
        image_id.push(255);

        let correct = &Risc0.is_correct_verifying_key(elf, &image_id).unwrap();

        assert!(!correct);
    }
    #[test]
    fn sp1_execute_can_correctly_execute_program() {
        let input = mock_contract_input_addr();
        let raw_input = input.abi_encode();

        let raw_result = &Sp1
            .execute(MOCK_CONSUMER_SP1_GUEST_ELF, &raw_input, &[], MOCK_CONSUMER_MAX_CYCLES)
            .unwrap();

        assert_eq!(*raw_result, mock_raw_output());
    }

    #[test]
    fn sp1_is_correct_verifying_key() {
        let (_, vk) = sp1_sdk::ProverClient::new().setup(MOCK_CONSUMER_SP1_GUEST_ELF);
        let mut vk_bytes = vk.hash_bytes().to_vec();

        let correct =
            &Sp1.is_correct_verifying_key(MOCK_CONSUMER_SP1_GUEST_ELF, &vk_bytes).unwrap();
        assert!(correct);

        vk_bytes.pop();
        vk_bytes.push(255);

        let correct =
            &Sp1.is_correct_verifying_key(MOCK_CONSUMER_SP1_GUEST_ELF, &vk_bytes).unwrap();
        assert!(!correct);
    }

    #[test]
    fn compare_vm_speeds() {
        const INTENSITY_TEST_RISC0_GUEST_ELF: &[u8] = include_bytes!(
            "../../../target/riscv-guest/riscv32im-risc0-zkvm-elf/release/intensity-test-guest"
        );
        const INTENSITY_TEST_SP1_GUEST_ELF: &[u8] =
            include_bytes!("../../../programs/sp1/intensity-test/elf/riscv32im-succinct-zkvm-elf");

        #[derive(BorshSerialize)]
        struct IntensityInput {
            hash_rounds: u32,
        }

        let input = IntensityInput { hash_rounds: 500 };
        let input_bytes = borsh::to_vec(&input).unwrap();

        // Test Risc0
        let start = std::time::Instant::now();
        let risc0_result = Risc0
            .execute(INTENSITY_TEST_RISC0_GUEST_ELF, &input_bytes, &[], HIGH_CYCLE_LIMIT)
            .unwrap();
        let risc0_time = start.elapsed();

        // Test SP1
        let start = std::time::Instant::now();
        let sp1_result =
            Sp1.execute(INTENSITY_TEST_SP1_GUEST_ELF, &input_bytes, &[], HIGH_CYCLE_LIMIT).unwrap();
        let sp1_time = start.elapsed();

        assert_eq!(risc0_result, sp1_result, "Full outputs don't match");

        let risc0_hash = &risc0_result[risc0_result.len() - 32..];
        let sp1_hash = &sp1_result[sp1_result.len() - 32..];
        assert_eq!(risc0_hash, sp1_hash, "Hash results don't match");

        println!("Risc0 time: {:?}", risc0_time);
        println!("SP1 time: {:?}", sp1_time);
        println!("Risc0 output length: {}", risc0_result.len());
        println!("SP1 output length: {}", sp1_result.len());
    }
}
