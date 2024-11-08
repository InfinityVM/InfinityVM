use sp1_helper::build_program_with_args;

fn main() {
    build_program_with_args(
        "../program",
        sp1_helper::BuildArgs {
            docker: false,
            output_directory: "../target/elf-compilation".to_string(),
            ..Default::default()
        },
    )
}
