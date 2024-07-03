# Rust workspace

The workspace configuration is defined by `rust/Cargo.toml`.

## Develop

**Workspace setup**

```sh
# set rust version
rustup use 1.79
rustup toolchain install nightly
```

```sh
# install protobuf compilation tools
brew update
brew install protobuf
```

**Iterating**

Lint

```
# you can run lint commands directly 
# lint
cargo clippy --fix

# format
cargo +nightly fmt

# or use the convience wrapper
make lint
```

Build

```sh
cargo build
```

Unit tests

```sh
cargo test
```


Integration tests

```sh
make test-all
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