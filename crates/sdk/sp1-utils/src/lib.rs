use cargo_metadata::MetadataCommand;
use ivm_zkvm::Zkvm;
use sp1_build::{build_program_with_args, BuildArgs};
use std::{fs, path::PathBuf};

pub fn build_sp1_program(elf_name: &str, program_dir: &str, output_dir: &str) {
    // We need to get the workspace root to convert all paths to absolute paths
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
    println!("cargo:rerun-if-changed={}", elf_path_abs.display());

    if let Ok(program_elf) = fs::read(&elf_path_abs) {
        let program_id = ivm_zkvm::Sp1.derive_program_id(&program_elf).unwrap();
        let program_id_path_abs = elf_path_abs.with_extension("vkey");

        bincode::serialize_into(
            fs::File::create(&program_id_path_abs).expect("Failed to create program ID file"),
            &program_id,
        )
        .expect("Failed to serialize program ID");

        println!("cargo:rerun-if-changed={}", program_id_path_abs.display());
    }
}

pub fn get_program_id(program_id_path: &str) -> [u8; 32] {
    // We need to get the workspace root to convert the program ID path to an absolute path
    let metadata = MetadataCommand::new().no_deps().exec().unwrap();
    let workspace_root = metadata.workspace_root;

    let program_id_path_abs = PathBuf::from(workspace_root).join(program_id_path);
    let program_id_bytes: Vec<u8> = bincode::deserialize_from(
        fs::File::open(program_id_path_abs).expect("Failed to open program ID file"),
    )
    .expect("Failed to deserialize program ID");
    program_id_bytes.try_into().expect("Invalid program ID length")
}
