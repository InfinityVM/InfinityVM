# Contributing

This repo is a rust workspace. The workspace configuration is defined by `Cargo.toml`.

## Develop

### Install tools

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

SP1

```sh
curl -L https://sp1.succinct.xyz | bash
sp1up
cargo prove --version
RUSTUP_TOOLCHAIN=succinct cargo --version
```

Foundry

```
curl -L https://foundry.paradigm.xyz | bash
foundryup
cd contracts && forge build
```

### Iterating

View rust docs (generated via cargo doc)

```sh
make doc
```

Build

```sh
cargo build

# to build a specific crate run
cargo build -p coprocessor-node # or whatever your crates name is
```

Lint

```sh
make clippy
make fmt

# or use the convenience wrapper
make lint
```

Unit tests

```sh
cargo test
```

E2E tests

```sh
make test-all
```

Re-geneerate proto based types

```sh
cargo run --bin proto-build
```

### VSCode

If you are working in VSCode, try installing the rust-analyzer extension. We recommend the following settings:

```
"rust-analyzer.rustfmt.extraArgs": ["+nightly"],
"[rust]": {
    "editor.formatOnSave": true,
    "editor.defaultFormatter": "rust-lang.rust-analyzer"
}
```

## Acknowledgements

- reth: the workspace configuration was largely inspired by [Reth][2].

[1]: https://doc.rust-lang.org/cargo/reference/workspaces.html#the-default-members-field
[2]: https://github.com/paradigmxyz/reth
