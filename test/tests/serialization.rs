use proto::{JobStatus, JobStatusType};

#[test]
#[ignore]
fn serde_json_test() {
    let input = vec![0, 0, 1];
    let job = proto::SubmitJobRequest {
        job: Some(proto::Job {
            id: input.clone(),
            nonce: 1,
            program_verifying_key: input.clone(),
            input: input.clone(),
            consumer_address: input.clone(),
            max_cycles: 100,
            request_signature: input.clone(),
            result: input.clone(),
            zkvm_operator_address: input.clone(),
            zkvm_operator_signature: input,
            status: JobStatus {
                status: JobStatusType::Unspecified as i32,
                failure_reason: None,
                retries: 0,
            },
        }),
    };

    let serialized = serde_json::to_string(&job).expect("serialization failed");
    let expected_json = r#"
        {
            "job": {
                "id": "AAAB",
                "nonce": 1,
                "programVerifyingKey": "AAAB",
                "input": "AAAB",
                "contractAddress": "000001",
                "maxCycles": 100,
                "requestSignature": "AAAB",
                "result": "AAAB",
                "zkvmOperatorAddress": "000001",
                "zkvmOperatorSignature": "AAAB",
                "status": {
                    "status": 0,
                    "failureReason": null,
                    "retries": 0
                }
            }
        }"#
    .replace(['\n', ' '], "");

    assert_eq!(serialized, expected_json);
}
