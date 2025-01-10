# Infinity Execution Client

`ivm-exec` is the Infinity execution client. It is built using reth and closely follows ethereum specs except in three edge cases:

1) It ignores timestamp checks in the Engine API. Timestamps are denominated in seconds and the Engine API dictates that the timestamp for an execution payload being requested must be greater then the previous execution payload. Ignoring checks allows us to request multiple execution payloads in the same second and thus achieve sub second block times. See [step 7](https://github.com/ethereum/execution-apis/blob/main/src/engine/paris.md#specification-1) of `engine_forkchoiceUpdatedV1` specifications for context.
2) Allow list for transaction sender and transaction `to` field value can be configured. This is intended for the bootstrap phase of the network where transactions will have zero gas fee, but will be tightly scoped to specific applications to reduce spamming.
3) The EVM logic is modified to not deduct gas from the caller or reward the coinbase.

## Usage

To run the node: `cargo run --bin ivm-exec -- node`.

### Configuration

The node has a standard reth CLI and uses the reth toml config.

#### Allow lists

The allow lists can be configured in an IVM config toml. If no config is found at start up, a restrictive default will get created in the data dir at `<DATADIR>/ivm_config.toml`. To specify a config at a different path you can run `cargo run --bin ivm-exec -- node --ivm-config <PATH>`.

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
NOTE: a transaction will be valid if it is a member `to` or a member of `sender`.

## Development

When developing against the node, it can be helpful to run the node in a development mode, where blocks are auto mined. We also have a `tx-allow.all` flag to override the IVM config and allow all transactions: 

```sh
cargo run --bin ivm-exec -- node \
  --dev \ # Run the node in dev mode
   --tx-allow.all \ # Allow all transactions
   --dev.block-time 2s \ # Set block times to 2 seconds
   --http.api="trace"  # Expose trace rpc endpoints
```

We have some simple scripts to send some transactions against the node:

```sh
cargo run --bin send-tx
```