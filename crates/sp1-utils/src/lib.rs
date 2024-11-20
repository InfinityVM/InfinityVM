use ivm_zkvm::Zkvm;
use sp1_build::{build_program_with_args, BuildArgs};
use std::{fs, path::PathBuf};

pub fn build_sp1_program(elf_name: &str, program_dir: &str, output_dir: &str) {
    let args = BuildArgs {
        elf_name: elf_name.to_string(),
        output_directory: output_dir.to_string(),
        ..Default::default()
    };
    build_program_with_args(program_dir, args);

    // This is a bit hacky but works.
    // Count directories after InfinityVM in current path to find number of parent dirs.
    let current_dir = std::env::current_dir().unwrap().to_string_lossy().to_string();
    let infinity_vm_pos = current_dir.rfind("InfinityVM").unwrap_or(0);
    let path_after_infinity = &current_dir[infinity_vm_pos..];
    let dir_count = path_after_infinity.matches('/').count();

    // Construct elf path relative to current dir with correct number of parent dirs.
    let elf_path_string =
        format!("{}/{}", "../".repeat(dir_count), output_dir.trim_start_matches("../"));
    let elf_path = PathBuf::from(elf_path_string).join(elf_name);

    println!("cargo:rerun-if-changed={}", elf_path.display());

    println!("cargo:warning=WOOOOO elf_path: {}", elf_path.display());
    println!("cargo:warning=WOOOOO current directory: {}", std::env::current_dir().unwrap().display());
    println!("cargo:warning=WOOOOO program_dir: {}", program_dir);
    println!("cargo:warning=WOOOOO output_dir: {}", output_dir);

    if let Ok(program_elf) = fs::read(&elf_path) {
        println!("cargo:warning=WOOOOO ELF DISCOVERED");
        let program_id = ivm_zkvm::Sp1.derive_program_id(&program_elf).unwrap();
        let program_id_path = elf_path.with_extension("vkey");

        bincode::serialize_into(
            fs::File::create(&program_id_path).expect("Failed to create program ID file"),
            &program_id,
        )
        .expect("Failed to serialize program ID");

        println!("cargo:rerun-if-changed={}", program_id_path.display());
    } else {
        println!("cargo:warning=WOOOOO ELF NOT DISCOVERED");
    }
}

pub fn get_program_id(program_id_path: &str) -> [u8; 32] {
    let program_id_bytes: Vec<u8> = bincode::deserialize_from(
        fs::File::open(program_id_path).expect("Failed to open program ID file"),
    )
    .expect("Failed to deserialize program ID");
    program_id_bytes.try_into().expect("Invalid program ID length")
}
