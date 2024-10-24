# Load testing

This doc contains instructions on how to run load testing for the coprocessor node. We use [Goose](https://book.goose.rs/title-page.html) for load tests.

## Setup (if running load tests against a local instance)

We need to first run the local setup for anvil + coprocessor node + REST gRPC gateway (this also deploys the required contracts and submits the mock consumer and clob ELF):
```
cargo run --bin local
```

## Running the load tests

There are two load test scenarios we run:

1. `LoadtestGetResult`: Starts by submitting an offchain `MockConsumer` job with nonce `1` and then each user keeps sending `GetResult` requests (no wait time between requests).
2. `LoadtestSubmitJob`: Each user sends a `SubmitJob` request to the coprocessor for an offchain `IntensityTest` job, using a specified intensity level. The job submission includes a nonce and intensity parameters, with an option to wait for job completion. The intensity parameters is defined by `INTENSITY_LEVEL`. See more details below.  

For each load test, Goose spawns multiple users, with a thread for each user.

To run the load tests:
```
cargo run --bin load-test --release
```

To stop the load tests, use `ctrl+C` (or, you can use the `RUN_TIME` env var mentioned in the next section). The results of the load tests will be saved in a `report.html` file. This contains stats and graphs on number of requests, response time, errors, etc.

## Load test parameters

We have a few parameters we can set in `load-test/.env` (an `example.env` is provided too):

1. `COPROCESSOR_GATEWAY_IP`: External IP of the coprocessor node REST gRPC gateway (defaults to localhost)
2. `COPROCESSOR_GATEWAY_PORT`: Port of the coprocessor node REST gRPC gateway
3. `ANVIL_IP`: External IP of the anvil instance (defaults to localhost)
4. `ANVIL_PORT`: Port of the anvil instance
5. `CONSUMER_ADDRESS`: Contract address of the mock consumer. If the `CONSUMER_ADDRESS` env var isn't set, the load test program will use the consumer address from the deploy info, or else a default consumer address hard-coded in the program.
6. `MAX_CYCLES`: Max cycles requested when submitting a job request to the coprocessor node.
7. `NUM_USERS`: Number of users spawned in the load tests
8. `REPORT_FILE_NAME`: Filename of the report with results (defaults to `report.html`)
9. `STARTUP_TIME`: Time required for all users to be spawned when starting tests
10. `RUN_TIME`: Time for which we want the load tests to run (in addition to the `STARTUP_TIME`)
11: `WAIT_UNTIL_JOB_COMPLETED`: Set to false to not poll for job results after submitting the job. Defaults to true.

There are other parameters that we can modify, these are detailed in the [Goose docs](https://book.goose.rs/getting-started/common.html).

## Specifying load-test program intensity

The guest program supports different levels of computation intensity. You can specify the intensity by setting the `INTENSITY_LEVEL` environment variable in the CLI by running:

`INTENSITY_LEVEL=light cargo run --bin load-test -- --scenarios LoadtestSubmitJob`


- `light`: 10 hashes 
  
- `medium`: 50 hashes

- `heavy`: 500 hashes

For each iteration, the program will do the specified number of hashes. The default intensity level is `medium`.

## Measuring time until job is completed

`LoadtestSubmitJob` only measures the response time between when a user sends a `SubmitJob` request and receives the `jobID` from the coprocessor node as a response. The coprocessor node then spawns a thread to actually execute the job.

We want to also measure the time between when a user sends `SubmitJob` and when the job is actually completed. We haven't been able to figure out a way to record this in the Goose report (Goose or Locust don't seem to support custom metrics), but for now we have added the option of logging the time it takes until the job is completed. If we want, we can direct these logs to a log file and perform analysis using that.

To enable this when running the load tests, set the `WAIT_UNTIL_JOB_COMPLETED` env var to `true` in `load-test/.env`.
