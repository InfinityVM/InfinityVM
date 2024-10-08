# Coprocessor Node API

The coprocessor node exposes [gRPC](https://grpc.io/about/) endpoints for third party interaction. gRPC is a modern standard used widely in the web2 world. gRPC services are defined in [protobuf](https://protobuf.dev/overview/) and most major languages have production grade tooling to generate clients and servers based off of the protobuf definitions.

All the protobuf definitions can be found [here](https://github.com/InfinityVM/InfinityVM/blob/44bd7f645b2b6170be5421770977a9f5e2f849ba/proto/coprocessor_node/v1/coprocessor_node.proto#L9).

Additionally, there is an embedded [json HTTP gateway](https://github.com/InfinityVM/InfinityVM/blob/main/crates/coprocessor-node/src/gateway.rs) with endpoints mapping 1:1 with each gRPC endpoint. This is useful when using tooling, (e.g. load testing frameworks), that does not support gRPC out of the box.

To learn more about how to use the gRPC API to build an application, take a look at the [Integration section](../integration/README.md).

## Clients

The InfinityVM team ships a Rust gRPC client, which can be found in the [`proto` crate](https://github.com/InfinityVM/InfinityVM/blob/44bd7f645b2b6170be5421770977a9f5e2f849ba/crates/sdk/proto/src/coprocessor_node.v1.rs#L236). As mentioned above, gRPC clients can be generated in other languages based off of the protobuf definitions.

## Endpoints

### `/coprocessor_node.v1.CoprocessorNode/SubmitJob`

#### Description

Async endpoint to submit a coprocessing job. To get the execution result, the user will need to poll `GetResult`. The result is also submitted onchain.

#### Details

HTTP gateway URL: `/v1/coprocessor_node/submit_job`

Request

```rust
pub struct SubmitJobRequest {
    /// ABI-encoded offchain job request
    pub request: Vec<u8>,
    /// Signature on ABI-encoded offchain job request
    pub signature: Vec<u8>,
    /// Value of offchain input passed into program (this isn't signed over)
    pub offchain_input: Vec<u8>,
    /// Value of state passed into program (this isn't signed over)
    pub state: Vec<u8>,
}
```

Response

```rust
pub struct SubmitJobResponse {
    pub job_id: Vec<u8>,
}
```


### `/coprocessor_node.v1.CoprocessorNode/GetResult`

#### Description

Get the result of a coprocessing job that was previously requested.

#### Details

HTTP gateway URL: `/v1/coprocessor_node/get_result`

Request

```rust
pub struct GetResultRequest {
    pub job_id: Vec<u8>,
}
```

Response

```rust
pub struct GetResultResponse {
    pub job_result: Option<JobResult>,
}

pub struct JobResult {
    /// The job ID (hash of nonce and consumer address)
    pub id: Vec<u8>,
    /// Nonce of the job request for a given consumer contract
    pub nonce: u64,
    /// CPU cycle limit for job execution
    pub max_cycles: u64,
    /// Address of consumer contract to receive this result. The address is 20 bytes (no zero padding).
    pub consumer_address: Vec<u8>,
    /// The ZK program verification key
    pub program_id: Vec<u8>,
    /// Program execution input posted onchain
    pub onchain_input: Vec<u8>,
    /// Hash of execution input posted offchain (DA)
    pub offchain_input_hash: Vec<u8>,
    /// Hash of program state
    pub state_hash: Vec<u8>,
    /// Signature on the offchain job request
    pub request_signature: Vec<u8>,
    /// ABI-encoded result of job execution with metadata
    /// tuple(JobID,OnchainInputHash,StateHash,MaxCycles,ProgramID,RawOutput)
    pub result_with_metadata: Vec<u8>,
    /// The signature of the operator that executed the job
    pub zkvm_operator_signature: Vec<u8>,
    /// The status of the job.
    pub status: Option<JobStatus>,
    /// Tx hash of relayed result
    pub relay_tx_hash: Vec<u8>,
}

pub enum JobStatusType {
    Unspecified = 0,
    Pending = 1,
    Done = 2,
    Failed = 3,
    Relayed = 4,
}
```

### `/coprocessor_node.v1.CoprocessorNode/SubmitProgram`

#### Description

Submit an ELF file for a zkVM program. This endpoint will validate the ELF and persist it in the network.

#### Details

HTTP gateway URL: `/v1/coprocessor_node/submit_program`

Request

```rust
pub struct SubmitProgramRequest {
    /// The compiled zkVM program ELF.
    pub program_elf: Vec<u8>,
    /// Type of ZKVM to execute.
    pub vm_type: i32,
}
```

Response

```rust
pub struct SubmitProgramResponse {
    /// The ZK program verification key.
    pub program_id: Vec<u8>,
}
```