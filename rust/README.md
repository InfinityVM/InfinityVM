# Rust workspace

The workspace configuration is defined by `rust/Cargo.toml`.
This is a PoC for our ZKVM Execution Service. The service is in charge of executing ZKVM programs.

## Develop

```sh
# set rust version
rustup use 1.79
```

```sh
# install protobuf compilation tools
brew update
brew install protobuf
```

```sh
# build the workspace
cargo build
```

## TODO

- [ ] cli that you can use ecdsa signing key with
- [ ] locally generate proto bindings so they are easier to understand, but git ignore them
- [ ] do we want the errors to be in the proto message so its not just a gRPC status?
- [ ] basic logging with debug + info
- [ ] prometheus metrics
- [ ] disable default features on all deps and pin versions

## Acknowledgements

- reth: the workspace configuration was largely inspired by reth.