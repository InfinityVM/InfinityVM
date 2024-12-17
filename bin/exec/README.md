# Infinity Execution Client

`ivm-exec` is the Infinity execution client. It is built using reth and closely follows ethereum specs except in two edge cases:

1) It ignores timestamp checks in the Engine API. Timestamps are denominated in seconds and the Engine API dictates that the timestamp for an execution payload being requested must be greater then the previous execution payload. Ignoring checks allows us to request multiple execution payloads in the same second and thus achieve sub second block times.
2) Allow list for transaction sender and transaction `to` field value can be configured. This is intended for the bootstrap phase of the network where transactions will have zero gas fee, but will be tightly scoped to specific applications to reduce spamming.

## Usage

To run the node: `cargo run --bin ivm-exec -- node`.

### Configuration

The node has a standard reth CLI and uses the reth toml config.

#### Allow lists

The allow lists can be configured in an IVM config toml. If no config is found at start up, a permissible default will get created in the data dir at `<DATADIR>/ivm_config.toml`. To specify a config at a different path you can run `cargo run --bin ivm-exec -- node --ivm-config <PATH>`.

The toml config file should look like 

```toml
[transaction_allow]
# Allow all transactions regardless of allow lists
all = false
# Allow transactions with to field that contain one of these addresses
to = ["0x1111111111111111111111111111111111111111"]
# Allow transactions signed by one of these addresses
sender = ["0x2222222222222222222222222222222222222222", "0x1234567890123456789012345678901234567890"]
```