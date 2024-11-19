use sp1_utils::build_sp1_program;

fn main() {
    build_sp1_program(
        "intensity-test-sp1-guest",
        "program",
        "programs/sp1/intensity-test/program/elf",
    );
}
