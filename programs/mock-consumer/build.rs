use sp1_utils::build_sp1_program;

fn main() {
    build_sp1_program("mock-consumer-sp1-guest", "program/", "programs/mock-consumer/program/elf/");
}
