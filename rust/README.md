# Rust workspace

The workspace configuration is defined by `rust/Cargo.toml`.

## Develop

**Workspace setup**

Rust

```sh
# set rust version
rustup use 1.79
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

**Iterating**

View docs

```sh
make doc
```

Build

```sh
cargo build
```

Lint

```sh
# NOTE: there is a bug with risc0 build tooling where you need to build before
# running clippy; needs investigation, but we might drop risc0 soon.
RISC0_SKIP_BUILD=true cargo clippy --fix
cargo +nightly fmt

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

To change Rust log level
```sh
export RUST_LOG="<log-level>"
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

- reth: the workspace configuration was largely inspired by reth.