# Run a Coprocessor Node

## Docker

## Build from source

## Configuration

### Logs

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