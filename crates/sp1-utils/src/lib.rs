use sp1_build::{build_program_with_args, BuildArgs};
use sp1_sdk::{HashableKey, ProverClient};
use std::{fs, path::PathBuf};

pub fn build_sp1_program(elf_name: &str, program_dir: &str, output_dir: &str) {
    let args = BuildArgs {
        elf_name: elf_name.to_string(),
        output_directory: output_dir.to_string(),
        ..Default::default()
    };
    build_program_with_args(program_dir, args);

    let elf_path = PathBuf::from(program_dir).join(output_dir).join(elf_name);

    println!("cargo:rerun-if-changed={}", elf_path.display());

    if let Ok(program_elf) = fs::read(&elf_path) {
        let (_, program_id) = ProverClient::new().setup(&program_elf);
        let program_id_bytes = program_id.hash_bytes().to_vec();
        let program_id_path = elf_path.with_extension("vkey");

        bincode::serialize_into(
            fs::File::create(&program_id_path).expect("Failed to create program ID file"),
            &program_id_bytes,
        )
        .expect("Failed to serialize program ID");

        println!("cargo:rerun-if-changed={}", program_id_path.display());
    }
}

pub fn get_program_id(program_id_path: &str) -> [u8; 32] {
    let program_id_bytes: Vec<u8> =
        bincode::deserialize_from(fs::File::open(program_id_path).expect("Failed to open program ID file"))
            .expect("Failed to deserialize program ID");
    program_id_bytes.try_into().expect("Invalid program ID length")
}
