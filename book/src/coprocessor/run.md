# Run a Coprocessor Node

## Docker

We do not yet have a public docker repository, but you can build [this docker image](https://github.com/InfinityVM/InfinityVM/blob/main/Dockerfile.coproc-node) located in the InfinityVM repo.

## Build from source

To build from source, you will need `rustc` >= 1.79. We recommend building with the release profile. To build with cargo, from the root of the [InfinityVM repo](https://github.com/InfinityVM/InfinityVM/tree/main) you can run `cargo build --bin coprocessor-node --release`.

## Configuration

The coprocessor node is primarily configured via CLI and optionally has some env vars to override CLI flags.

The CLI has the following options:

```sh
Usage: coprocessor-node [OPTIONS] --job-manager-address <JOB_MANAGER_ADDRESS> [COMMAND]

Commands:
  dev        Use a development key
  key-store  Use an encrypted keystore
  secret     Pass the hex encoded secret in at the command line
  help       Print this message or the help of the given subcommand(s)

Options:
      --grpc-address <GRPC_ADDRESS>
          gRPC server address [default: 127.0.0.1:50051]
      --http-address <HTTP_ADDRESS>
          Address to listen on for the REST gRPC gateway [default: 127.0.0.1:8080]
      --prom-address <PROM_ADDRESS>
          prometheus metrics address [default: 127.0.0.1:3001]
      --job-manager-address <JOB_MANAGER_ADDRESS>
          `JobManager` contract address
      --http-eth-rpc <HTTP_ETH_RPC>
          HTTP Ethereum RPC address. Defaults to a local anvil node address [default: http://127.0.0.1:8545]
      --ws-eth-rpc <WS_ETH_RPC>
          WS Ethereum RPC address. Defaults to a local anvil node address [default: ws://127.0.0.1:8545]
      --ws-backoff-limit-ms <WS_BACKOFF_LIMIT_MS>
          WS Ethereum RPC retry backoff duration limit in milliseconds [default: 300000]
      --ws-backoff-multiplier-ms <WS_BACKOFF_MULTIPLIER_MS>
          WS Ethereum RPC retry backoff multiplier. The sleep duration will be `num_retrys * backoff_multiplier_ms` [default: 10]
      --chain-id <CHAIN_ID>
          Chain ID of where results are expected to get submitted. Defaults to anvil node chain id [default: 31337]
      --db-dir <DB_DIR>
          Path to the directory to include db [default: $HOME/.config/ivm/networks/ivm-dev0/coprocessor-node/db]
      --worker-count <WORKER_COUNT>
          Number of worker threads to use for processing jobs [default: 4]
      --max-retries <MAX_RETRIES>
          Max number of retries for relaying a job [default: 3]
      --exec-queue-bound <EXEC_QUEUE_BOUND>
          Max size for the exec queue [default: 256]
      --job-sync-start <JOB_SYNC_START>
          Block to start syncing from [default: earliest]
      --confirmations <CONFIRMATIONS>
          Required confirmations for tx [default: 1]
  -h, --help
          Print help
  -V, --version
          Print version
```

The following environment variables are supported and override the CLI private key subcommands:

```sh
# ECDSA private key for submitting coprocessing results on chain.
RELAYER_PRIVATE_KEY="<hex encoded private key>"

# ECDSA private key for signing coprocessing results. 
# This key is registered as belonging to the coprocessor operator.
ZKVM_OPERATOR_PRIVATE_KEY="<hex encoded private key>"
```

### Logs

The logging capabilities are entirely controlled by env vars.

To change Rust log level when running a binary:

```sh
RUST_LOG="<log-level>"
```

To change the Rust log format between text or json:

```sh
# Use "json" for JSON. Defaults to text.
RUST_LOG_FORMAT="text"  
```

To export logs to a file:

```sh
# Defaults to not exporting logs to a file
RUST_LOG_FILE="zkvm.log" 

# If this is not specified, logs will be written to the current directory "."
RUST_LOG_DIR="/path/to/log"
```
