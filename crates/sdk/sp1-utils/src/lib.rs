use cargo_metadata::MetadataCommand;
use ivm_zkvm::Zkvm;
use sp1_build::{build_program_with_args, BuildArgs};
use std::{fs, path::PathBuf};

pub fn build_sp1_program(elf_name: &str, program_dir: &str, output_dir: &str) {
    // Track the Cargo.toml and src directory of the program
    println!("cargo:rerun-if-changed={}/Cargo.toml", program_dir);
    println!("cargo:rerun-if-changed={}/src", program_dir);

    let metadata = MetadataCommand::new().no_deps().exec().unwrap();
    let workspace_root = metadata.workspace_root;

    let output_dir_abs = PathBuf::from(workspace_root).join(output_dir);
    let args = BuildArgs {
        elf_name: elf_name.to_string(),
        output_directory: output_dir_abs.to_string_lossy().to_string(),
        ..Default::default()
    };
    build_program_with_args(program_dir, args);

    let elf_path_abs = output_dir_abs.join(elf_name);
    if let Ok(program_elf) = fs::read(&elf_path_abs) {
        let program_id = ivm_zkvm::Sp1.derive_program_id(&program_elf).unwrap();
        let program_id_path_abs = elf_path_abs.with_extension("vkey");

        fs::write(&program_id_path_abs, program_id).expect("Failed to write program ID file");
    }
}
