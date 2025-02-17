use cargo_metadata::MetadataCommand;
use ivm_zkvm::Zkvm;
use sp1_build::{build_program_with_args, BuildArgs};
use std::{fs, path::PathBuf};

const SP1_SKIP_BUILD: &str = "SP1_SKIP_BUILD";

pub fn build_sp1_program(elf_name: &str, program_dir: &str, output_dir: &str) {
    if std::env::var(SP1_SKIP_BUILD).unwrap_or_default() != String::default() {
        return;
    }

    let metadata = MetadataCommand::new().no_deps().exec().unwrap();
    let workspace_root = metadata.workspace_root;
    let output_dir_abs = PathBuf::from(workspace_root).join(output_dir);
    let elf_path_abs = output_dir_abs.join(elf_name);
    let program_id_path_abs = elf_path_abs.with_extension("vkey");

    let args = BuildArgs {
        elf_name: Some(elf_name.to_string()),
        output_directory: Some(output_dir_abs.to_string_lossy().to_string()),
        ..Default::default()
    };
    build_program_with_args(program_dir, args);

    if let Ok(program_elf) = fs::read(&elf_path_abs) {
        let program_id = ivm_zkvm::Sp1.derive_program_id(&program_elf).unwrap();

        fs::write(&program_id_path_abs, program_id).expect("Failed to write program ID file");
    }
}
