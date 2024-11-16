use sp1_build::{build_program_with_args, BuildArgs};
use sp1_sdk::{HashableKey, ProverClient};
use std::{fs, path::PathBuf};

fn main() {
    let output_dir = "elf-compilation/riscv32im-succinct-zkvm-elf/release/";
    let elf_name = "intensity-test-sp1-guest";

    let args = BuildArgs {
        elf_name: elf_name.to_string(),
        output_directory: output_dir.to_string(),
        ..Default::default()
    };
    build_program_with_args("program", args);

    let elf_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../target")
        .join(output_dir)
        .join(elf_name);

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
