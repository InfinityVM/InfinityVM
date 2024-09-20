# Load testing

This doc contains instructions on how to run load testing for the coprocessor node. We use [Goose](https://book.goose.rs/title-page.html) for load tests.

## Setup (if running load tests against a local instance)

We need to first run the local setup for anvil + coprocessor node (this also deploys the required contracts and submits the mock consumer ELF):
```
cargo run --bin local
```

Next, we need to start the REST gRPC gateway (since Goose tests need to run against an HTTP server):
```
cargo run --bin http-gateway -- --grpc-address 127.0.0.1:50420
```

## Running the load tests

There are two load test scenarios we run:

1. `LoadtestSubmitJob`: Each user sends a `SubmitJob` request to the coprocessor for an offchain `MockConsumer` job, with a wait time of 1-3 secs between each job submission for a user.
2. `LoadtestGetResult`: Starts by submitting an offchain `MockConsumer` job with nonce `1` and then each user keeps sending `GetResult` requests (no wait time between requests).

For each load test, Goose spawns multiple users, with a thread for each user.

To run the load tests:
```
cargo run --bin test-load --release -- --host http://127.0.0.1:8080 --report-file=report.html
```
where `http://127.0.0.1:8080` is the address of the REST gRPC gateway.

If we're testing against a remote instance of the coprocessor node and gRPC gateway, we would pass the remote URL for `--host`.

We can also set `CONSUMER_ADDRESS` and `MAX_CYCLES` in a `.env` file in `test-load` (an `example.env` is provided). If the `CONSUMER_ADDRESS` env var isn't set, the load test program will use the consumer address from the deploy info, or else a default consumer address hard-coded in the program.

To stop the load tests, use `ctrl+C`. The results of the load tests will be saved in a `report.html` file. This contains stats and graphs on number of requests, response time, errors, etc.

## Load test parameters

By default, Goose will spawn 10 users. If we want to increase the number of users to 50, for example, we would use the `-u` flag:
```
cargo run --bin test-load --release -- --host http://127.0.0.1:8080 --report-file=report.html -u 50
```

There are other parameters (startup time, etc.) that we can play with, these are detailed in the [Goose docs](https://book.goose.rs/getting-started/common.html).

## Measuring time until job is completed

`LoadtestSubmitJob` only measures the response time between when a user sends a `SubmitJob` request and receives the `jobID` from the coprocessor node as a response. The coprocessor node then spawns a thread to actually execute the job.

We want to also measure the time between when a user sends `SubmitJob` and when the job is actually completed. We haven't been able to figure out a way to record this in the Goose report (Goose or Locust don't seem to support custom metrics), but for now we have added the option of logging the time it takes until the job is completed. If we want, we can direct these logs to a log file and perform analysis using that.

To enable this when running the load tests, set the `WAIT_UNTIL_JOB_COMPLETED` env var to `true` in `test-load/.env`.
