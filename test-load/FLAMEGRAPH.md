# Flamegraph on mac

[Flamegraph](https://github.com/flamegraph-rs/flamegraph?tab=readme-ov-file) is performance analysis tool.

## Setting up and running flamegraph
1. Install  flamegraph

```sh
cargo install flamegraph
```

2. Start the local env

Note: your might want to change the `--worker-count for the coproc-node to be 1.

```sh
cargo run --bin local
```

You should see the log line with the PID. Keep track of the pid

```sh
coproc-node process ID: <PID>
```

3. Start the load tests to generate realistic load

This sets it to no ramp up time and one user to keep things simple
```
export WAIT_UNTIL_JOB_COMPLETED=false
cargo run --bin test-load --release -- --scenarios loadtestsubmitjob -s 0 -t 45 -u 10
```

4. Run flamegraph. To create the graph, hit ctrl+c after the desired amount of time

```
sudo flamegraph -o my_flamegraph.svg --pid <PID>
```

5. Open the graph svg. You can do this by dragging `my_flamegraph.svg` to your google chrome

### Notes

The above shows how to run just the load test submit job scenario and removes `get_result` calls to more clearly examine where time is spent while executing. You may want to examine other aspects, in which case you may want to run a different load test or adjust parameters of the node (such as worker count).

- Our initial findings showed that the longest part of job execution is calculating the verifying key. We can improve benchmarks by commenting out the verification key derivation. Inside of the actual execution logic for risc0, the longest part by far is deriving the verification key - however we don't have control over this logic.