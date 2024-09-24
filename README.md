# Infinity

Infinite possibilities.

The workspace configuration is defined by `Cargo.toml`.

## Develop

**Workspace setup**

Rust

```sh
# set rust version
rustup default 1.80.1
rustup toolchain install nightly
```

Proto

- MacOS

```sh
# install protobuf compilation tools
brew update
brew install protobuf
```

- Debian/Ubuntu

```sh
# install protobuf compilation tools
apt-get update
apt-get install install protobuf-compiler
```

Risc0

```sh
cargo install cargo-binstall
cargo binstall cargo-risczero
cargo risczero install
cargo risczero --version
```

Sp1

```sh
curl -L https://sp1.succinct.xyz | bash
sp1up
cargo prove --version
```

Foundry

```
curl -L https://foundry.paradigm.xyz | bash
foundryup
cd contracts && forge build
```

**Iterating**

View docs

```sh
make doc
```

Build

```sh
# N.B. this will only build the workspace's default-members
cargo build

# to build a specific crate run
cargo build -p coprocessor-node # or whatever your crates name is
```

Lint

```sh
# NOTE: there is a bug with risc0 build tooling where you need to build before
# running clippy; needs investigation, but we might drop risc0 soon.
make clippy
make fmt

# or use the convenience wrapper
make lint
```

Unit tests

```sh
cargo test
```

Integration tests

```sh
make test-all
```

To change Rust log level when running a binary

```sh
export RUST_LOG="<log-level>"
```

To change the Rust log format between text or json:

```sh
export RUST_LOG_FORMAT="text"  # Use "json" for JSON
```

To export logs to a file:

```sh
export RUST_LOG_FILE="zkvm.log" 
export RUST_LOG_DIR="/path/to/log" # Optional
```
If RUST_LOG_DIR is not specified, logs will be written to the current directory "."

To run any binary that is a non default member you need to specify the package:

```sh
cargo run -p ethos-reth --bin ethos-reth
```

Note: we leverage the [workspace.default-members][1] config the reduce the amount of code compiled by default when iterating

### VSCode

If you are working in VSCode, try installing the rust-analyzer extension. We recommend the following settings:

```
"rust-analyzer.rustfmt.extraArgs": ["+nightly"],
"[rust]": {
    "editor.formatOnSave": true,
    "editor.defaultFormatter": "rust-lang.rust-analyzer"
}
```

## Coprocessor Architecture Diagram

[Coprocessor Architecture Diagram](coprocessor_architecture.png)

<!-- https://app.excalidraw.com/s/8oh7cYrMkAR/5fsQ8hJAP0k -->

## Acknowledgements

- reth: the workspace configuration was largely inspired by [Reth][2].

[1]: https://doc.rust-lang.org/cargo/reference/workspaces.html#the-default-members-field
[2]: https://github.com/paradigmxyz/reth
