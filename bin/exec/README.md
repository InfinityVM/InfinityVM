# Infinity Execution Client

TODO(zeke): update

`ivm-exec` is the Infinity execution client. It is built using reth and closely follows ethereum specs except in four edge cases:

1) It ignores timestamp checks in the Engine API. Timestamps are denominated in seconds and the Engine API dictates that the timestamp for an execution payload being requested must be greater than the previous execution payload. Ignoring checks allows us to request multiple execution payloads in the same second and thus achieve sub second block times. See [step 7](https://github.com/ethereum/execution-apis/blob/main/src/engine/paris.md#specification-1) of `engine_forkchoiceUpdatedV1` specifications for context.
2) Allow list for transaction sender and transaction `to` (recipient) field value can be configured. This is intended for the bootstrap phase of the network where transactions will have zero gas fee, but will be tightly scoped to specific applications to reduce spamming. At the moment this is only affects tx pool validation and payload building; in the future it may be enforced in the execution layer and thus become consensus critical. 
3) The EVM logic is modified to not deduct gas from the caller or reward the coinbase.
4) The payload builder ignores priority fees and instead prioritizes based on if a sender is configured as a priority sender and how long the transaction has been in the pool. Priority senders always take precedence, then transaction time in pool (FIFO). This is not consensus critical.

## Usage

To run the node: `cargo run --bin ivm-exec -- node`.

### Configuration

The node has a standard reth CLI and uses the reth toml config.

#### Allow lists

The allow lists can be configured in an IVM config toml. If no config is found at start up, a permissive default will get created in the data dir at `<DATADIR>/ivm_config.toml`. To specify a config at a different path you can run `cargo run --bin ivm-exec -- node --ivm-config <PATH>`. In the future we will likely add defaults to the binary for major networks.

The toml config file should look like 

```toml
# IVM Configuration File

# Priority senders for payload building. This configuration is used to determine
# if the senders transaction should take priority when building a payload.
# This is not consensus related.
priority_senders = [
    "0x8888888888888888888888888888888888888888",
    "0x9999999999999999999999999999999999999999",
    "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
]

# A fork take effect at a given timestamp and is valid up until the next fork.
[[forks]]
# Fork activated at Unix timestamp 1672531200 (January 1, 2023 00:00:00 UTC)
activation_timestamp = 1672531200
[forks.allow_config]
all = false
# List of allowed recipient addresses
to = [
    "0x1234567890123456789012345678901234567890",
    "0xabcdefabcdefabcdefabcdefabcdefabcdefabcd"
]
# List of allowed sender addresses
sender = [
    "0x2222222222222222222222222222222222222222",
    "0x3333333333333333333333333333333333333333"
]

[[forks]]
# Fork activated at Unix timestamp 1688169600 (July 1, 2023 00:00:00 UTC)
activation_timestamp = 1688169600
[forks.allow_config]
all = false
to = [
    "0x4444444444444444444444444444444444444444",
    "0x5555555555555555555555555555555555555555"
]
sender = [
    "0x6666666666666666666666666666666666666666",
    "0x7777777777777777777777777777777777777777"
]

[[forks]]
# Fork activated at Unix timestamp 1704067200 (January 1, 2024 00:00:00 UTC)
activation_timestamp = 1704067200
[forks.allow_config]
all = true
to = []
sender = []
```
NOTE: a transaction will be valid if it is a member of `to` or a member of `sender`.

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
