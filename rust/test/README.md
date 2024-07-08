# Integration `test` crate

# Layout

`src/`: various utilities for running integration tests. 
`tests/`: files that contain tests. Each Rust source file in the tests directory is compiled as a separate crate. [You can learn more here.](https://doc.rust-lang.org/rust-by-example/testing/integration_testing.html)

# Please read before iterating

tl;dr: run `make test-all` to iterate on tests.

For integration tests, we mark them as `#[ignored]`. The tests directly rely on binaries in `rust/target/debug`, which require one to run `cargo build` to update. A common foot gun with this type of test is forgetting to rebuild the binaries while making changes. To avoid the foot gun, we require developers to thoughtfully opt in via adding the ignored flag: `cargo test -- --ignored`. In `rust/Makefile`, we have a convenience command `make test-all` that will run `cargo build && cargo test -- --ignored` for you