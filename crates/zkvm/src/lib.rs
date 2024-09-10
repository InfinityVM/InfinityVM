//! ZKVM trait and implementations. The trait should abstract over any complexities to specific VMs.

use alloy::primitives::keccak256;
use risc0_binfmt::compute_image_id;
use risc0_zkvm::{Executor, ExecutorEnv, LocalProver};
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
        raw_input: &[u8],
        max_cycles: u64,
    ) -> Result<Vec<u8>, Error>;

    /// Execute the program for a stateful job and return the raw output;
    fn execute_stateful(
        &self,
        program_elf: &[u8],
        raw_input: &[u8],
        program_state: &[u8],
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
        raw_input: &[u8],
        max_cycles: u64,
    ) -> Result<Vec<u8>, Error> {
        let env = ExecutorEnv::builder()
            .session_limit(Some(max_cycles))
            .write_slice(raw_input)
            .build()
            .map_err(|source| Error::Risc0 { source })?;

        let prover = LocalProver::new("locals only");

        let prove_info = prover.execute(env, program_elf).map_err(|source| {
            match source.downcast_ref::<&str>() {
                Some(&"Session limit exceeded") => Error::CycleLimitExceeded,
                _ => Error::Risc0 { source },
            }
        })?;

        Ok(prove_info.journal.bytes)
    }

    fn execute_stateful(
        &self,
        program_elf: &[u8],
        raw_input: &[u8],
        program_state: &[u8],
        max_cycles: u64,
    ) -> Result<Vec<u8>, Error> {
        let state_len = program_state.len() as u32;
        let input_len = raw_input.len() as u32;

        let env = ExecutorEnv::builder()
            .session_limit(Some(max_cycles))
            .write(&state_len)
            .map_err(|source| Error::Risc0 { source })?
            .write_slice(program_state)
            .write(&input_len)
            .map_err(|source| Error::Risc0 { source })?
            .write_slice(raw_input)
            .build()
            .map_err(|source| Error::Risc0 { source })?;

        let prover = LocalProver::new("locals only");
        let prove_info =
            prover.execute(env, program_elf).map_err(|source| Error::Risc0 { source })?;

        Ok(prove_info.journal.bytes)
    }
}

// TODO: https://github.com/Ethos-Works/InfinityVM/issues/120
// Sp1 impl of [Zkvm].
// #[derive(Debug)]
// pub struct Sp1;
// impl Zkvm for Sp1 {
//     fn derive_verifying_key(&self, program_elf: &[u8]) -> Result<Vec<u8>, Error> {
//         let (_, vk) = ProverClient::new().setup(program_elf);

//         Ok(vk.hash_bytes().to_vec())
//     }

//     fn execute(
//         &self,
//         program_elf: &[u8],
//         raw_input: &[u8],
//         max_cycles: u64,
//     ) -> Result<Vec<u8>, Error> {
//         let mut stdin = SP1Stdin::new();
//         stdin.write_slice(raw_input);

//         let client = ProverClient::new();
//         let (public_values, _) = client
//             .execute(program_elf, stdin)
//             .max_cycles(max_cycles)
//             .run()
//             .map_err(|source| Error::Sp1 { source })?;

//         Ok(public_values.to_vec())
//     }
// }

#[cfg(test)]
mod test {
    use crate::{Risc0, Zkvm};
    use alloy::{rlp::Decodable, sol_types::SolType};
    use vapenation_core::{VapeNationArg, VapeNationMetadata};

    const VAPENATION_ELF_PATH: &str =
        "../../target/riscv-guest/riscv32im-risc0-zkvm-elf/release/vapenation-guest";

    // TODO: https://github.com/Ethos-Works/InfinityVM/issues/120
    // const VAPENATION_ELF_SP1_PATH: &str =
    //     "../../programs/sp1/vapenation/program/elf/riscv32im-succinct-zkvm-elf";

    #[test]
    fn risc0_execute_can_correctly_execute_program() {
        let vapenation_elf = std::fs::read(VAPENATION_ELF_PATH).unwrap();

        let input = 2u64;
        let raw_input = VapeNationArg::abi_encode(&input);

        let max_cycles = 32 * 1024 * 1024;
        let raw_result = &Risc0.execute(&vapenation_elf, &raw_input, max_cycles).unwrap();

        let metadata = VapeNationMetadata::decode(&mut &raw_result[..]).unwrap();
        let phrase = (0..2).map(|_| "NeverForget420".to_string()).collect::<Vec<_>>().join(" ");

        assert_eq!(metadata.nation_id, 352380);
        assert_eq!(metadata.points, 5106);
        assert_eq!(metadata.phrase, phrase);
    }

    #[test]
    fn risc0_is_correct_verifying_key() {
        let vapenation_elf = std::fs::read(VAPENATION_ELF_PATH).unwrap();
        let mut image_id =
            risc0_binfmt::compute_image_id(&vapenation_elf).unwrap().as_bytes().to_vec();

        let correct = &Risc0.is_correct_verifying_key(&vapenation_elf, &image_id).unwrap();
        assert!(correct);

        image_id.pop();
        image_id.push(255);

        let correct = &Risc0.is_correct_verifying_key(&vapenation_elf, &image_id).unwrap();

        assert!(!correct);
    }

    // TODO: https://github.com/Ethos-Works/InfinityVM/issues/120
    // #[test]
    // fn sp1_execute_can_correctly_execute_program() {
    //     let vapenation_elf = std::fs::read(VAPENATION_ELF_SP1_PATH).unwrap();

    //     let input = 2u64;
    //     let raw_input = VapeNationArg::abi_encode(&input);

    //     let max_cycles = 32 * 1024 * 1024;
    //     let raw_result = &Sp1.execute(&vapenation_elf, &raw_input, max_cycles).unwrap();

    //     let metadata = VapeNationMetadata::decode(&mut &raw_result[..]).unwrap();
    //     let phrase = (0..2).map(|_| "NeverForget420".to_string()).collect::<Vec<_>>().join(" ");

    //     assert_eq!(metadata.nation_id, 352380);
    //     assert_eq!(metadata.points, 5106);
    //     assert_eq!(metadata.phrase, phrase);
    // }

    // #[test]
    // fn sp1_is_correct_verifying_key() {
    //     let vapenation_elf = std::fs::read(VAPENATION_ELF_SP1_PATH).unwrap();
    //     let client = sp1_sdk::ProverClient::new();

    //     // setup
    //     let (_, vk) = client.setup(vapenation_elf.as_slice());
    //     let mut image_id = vk.hash_bytes().as_slice().to_vec();

    //     let correct = &Sp1.is_correct_verifying_key(&vapenation_elf, &image_id).unwrap();
    //     assert!(correct);

    //     image_id.pop();
    //     image_id.push(255);

    //     let correct = &Sp1.is_correct_verifying_key(&vapenation_elf, &image_id).unwrap();
    //     assert!(!correct);
    // }
}
