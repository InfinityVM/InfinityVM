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

/// Something that can execute programs and generate ZK proofs for them.
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

/// Risc0 impl of [Zkvm].
#[derive(Debug)]
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

        Ok(prove_info.journal.bytes)
    }
}

#[cfg(test)]
mod test {
    use crate::{Risc0, Zkvm};
    use alloy_rlp::Decodable;
    use alloy_sol_types::SolType;
    use vapenation_core::{VapeNationArg, VapeNationMetadata};

    const VAPENATION_ELF_PATH: &str =
        "../../target/riscv-guest/riscv32im-risc0-zkvm-elf/release/vapenation_guest";

    #[test]
    fn risc0_execute_can_correctly_execute_program() {
        let vapenation_elf = std::fs::read(VAPENATION_ELF_PATH).unwrap();

        let input = 2u64;
        let raw_input = VapeNationArg::abi_encode(&input);

        let max_cycles = 32 * 1024 * 1024;
        let raw_result = Risc0::execute(&vapenation_elf, &raw_input, max_cycles).unwrap();

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

        let correct = Risc0::is_correct_verifying_key(&vapenation_elf, &image_id).unwrap();
        assert!(correct);

        image_id.pop();
        image_id.push(255);

        let correct = Risc0::is_correct_verifying_key(&vapenation_elf, &image_id).unwrap();

        assert!(!correct);
    }
}
