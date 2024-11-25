use ivm_sp1_utils::build_sp1_program;

fn main() {
    let current_dir = std::env::current_dir().unwrap();
    println!("cargo:rerun-if-changed={}", current_dir.join("src/lib.rs").display());
    println!("cargo:rerun-if-changed={}", current_dir.join("Cargo.toml").display());

    println!("cargo:rerun-if-changed={}", current_dir.join("program/src/main.rs").display());
    println!("cargo:rerun-if-changed={}", current_dir.join("program/Cargo.toml").display());

    build_sp1_program("matching-game-sp1-guest", "program/", "target/sp1/matching-game/");
}
