use sp1_build::{build_program_with_args, BuildArgs};

fn main() {
    let args = BuildArgs {
        elf_name: "matching-game-sp1-guest".to_string(),
        output_directory: "elf-compilation/riscv32im-succinct-zkvm-elf/release/".to_string(),
        ..Default::default()
    };
    build_program_with_args("program", args);
}
