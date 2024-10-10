# Offchain Jobs

Offchain jobs enable you to send job requests directly to the InfinityVM coprocessor. This is done using the coprocessor node's gRPC or REST API. The result of executing a job is still submitted onchain to your app contract, similar to onchain jobs.

## High-level flow

The flow for a simple offchain job is:

1. User or app sends a job request using the coprocessor node's gRPC or REST API. This involves sending a signature over the job request as well.
2. The InfinityVM coprocessor executes your zkVM program with the inputs from the job request.
3. The coprocessor posts the result of executing the job onchain, and this can now be used by the app contract. 

**Note:** For offchain jobs, the coprocessor also posts the job request onchain. This is because the InfinityVM contracts need to verify that the metadata which the coprocessor commits to when posting the result (program ID, input, etc.) matches the metadata in the job request. This isn't required for onchain jobs since the job request happens onchain anyway.

#### App Servers

If your app sends job requests, there is an interesting class of applications that run as real-time servers. More info on this in the [`App Servers`](./offchain.md#app-servers-1) section below. 

## gRPC/REST endpoints

### SubmitJob

A user or an app can send a job request directly to the InfinityVM coprocessor using the [`SubmitJob` endpoint](../coprocessor/api.md#coprocessor_nodev1coprocessornodesubmitjob):

```rust,ignore
// SubmitJobRequest defines the request structure to submit a job to the
// coprocessing coprocessor_node.
message SubmitJobRequest {
  bytes request = 1; // ABI-encoded offchain job request
  bytes signature = 2; // Signature on ABI-encoded offchain job request
  bytes offchain_input = 3; // Value of offchain input passed into program (this isn't signed over)
  bytes state = 4; // Value of state passed into program (this isn't signed over)
}
```

This includes the actual job request (ABI-encoded), a signature over the request, and `offchain_input` and `state` (we explain `offchain_input` and `state` later in this doc). The job request is an ABI-encoded version of this:
```rust,ignore
struct OffchainJobRequest {
    uint64 nonce;
    uint64 maxCycles; // Max number of cycles to execute program in zkVM
    address consumer;
    bytes programID;
    bytes onchainInput;
    bytes32 offchainInputHash;
    bytes32 stateHash;
}
```
A few notes:
- `consumer`: address of your app contract.
- `programId`: unique ID of your program. You can get the program ID when you submit your program to the coprocessor node's [`SubmitProgram` endpoint](../coprocessor/api.md#coprocessor_nodev1coprocessornodesubmitprogram).
- `maxCycles`: max number of cycles to execute your program in zkVM
- `nonce`: each job request for a particular app contract has a unique nonce, to prevent replay attacks. More info on this in [`Writing your app contract`](./offchain.md#writing-your-app-contract) below.

The `SubmitJob` endpoint returns a **unique Job ID** for the job.

### GetResult

Once the InfinityVM coprocessor executes your zkVM program with the inputs, it will submit the result onchain, but you can also query the result offchain if you'd like. You can use the coprocessor node's [`GetResult endpoint`](../coprocessor/api.md#coprocessor_nodev1coprocessornodegetresult). This takes in the job ID as input and returns the job result + metadata (program ID, job status, inputs for program, etc.).

## Onchain vs offchain input



## Writing your app contract

#### Nonces

## App Servers

![app servers](../assets/app-servers.png)

## Testing your app


- what are offchain jobs
- submitjob and getresult endpoints
- nonces
- high level flow of posting result and signed job request onchain
- onchain vs offchain input
- writing your app contract (interfaces isValidSignature() offchainrequester singleoffchainsigner etc.)
- basic testing using infinity foundry template
- app server idea and stateful offchain jobs (with high-level diagram of STF) and state hash in job request and statefulconsumer interface




Offchain job requests are are triggered by sending a request directly to a coprocessor node's [`SubmitJob` endpoint](../coprocessor/api.md#coprocessor_nodev1coprocessornodesubmitjob). The result will be submitted onchain and can also be queried directly from the coprocessor node via the [`GetResult` endpoint](../coprocessor/api.md#coprocessor_nodev1coprocessornodegetresult).

Offchain job requests can either be user initiated or service initiated. In the latter, there is a class of applications that run as real time servers with core state transition function (STFs) packaged up in InfinityVM programs. The servers will process user requests real time and have a background task that regularly batches STF inputs and submits them to the coprocessor node as a Job request. The results and are then submitted onchain and immediately usable by all other applications. The CLOB example illustrate

These servers can accept and process user requests in real-time, and regularly batch inputs and submit them to the InfinityVM coprocessor as an offchain request. You can write some state transition function in your zkVM program which performs compute on each batch of inputs. The results are finally submitted onchain and immediately usable by the app. An example of this is shown in the [Offchain Example: CLOB](./clob.md) section.
