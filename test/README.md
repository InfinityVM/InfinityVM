# End to End `test` crate

# Layout

`src/`: various utilities for running integration tests. 
`tests/`: files that contain tests. Each Rust source file in the tests directory is compiled as a separate crate. [You can learn more here.](https://doc.rust-lang.org/rust-by-example/testing/integration_testing.html)

# Please read before iterating

tl;dr: run `make test-all` to iterate on tests.

The e2e tests are ignored because they take awhile to run. `cargo test -- --ignored`. In `Makefile`, we have a convenience command `make test-all` that will run `cd contracts && forge build && cargo test -- --ignored` for you