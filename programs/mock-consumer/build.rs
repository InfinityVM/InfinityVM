use ivm_sp1_utils::build_sp1_program;

fn main() {
    build_sp1_program("mock-consumer-sp1-guest", "program/", "target/sp1/mock-consumer/");
}
