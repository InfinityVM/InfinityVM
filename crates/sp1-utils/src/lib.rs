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
        let (_, vk) = ProverClient::new().setup(&program_elf);
        let vk_bytes = vk.hash_bytes().to_vec();
        let vk_path = elf_path.with_extension("vkey");

        bincode::serialize_into(
            fs::File::create(&vk_path).expect("Failed to create vkey file"),
            &vk_bytes,
        )
        .expect("Failed to serialize verifying key");

        println!("cargo:rerun-if-changed={}", vk_path.display());
    }
}

pub fn get_elf_id(vkey_path: &str) -> [u8; 32] {
    let vk_bytes: Vec<u8> =
        bincode::deserialize_from(fs::File::open(vkey_path).expect("Failed to open vkey file"))
            .expect("Failed to deserialize verifying key");
    vk_bytes.try_into().expect("Invalid verifying key length")
}
