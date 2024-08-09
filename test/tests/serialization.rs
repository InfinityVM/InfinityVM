use proto::{JobStatus, JobStatusType};

#[test]
#[ignore]
fn serde_json_test() {
    let input = vec![0, 0, 1];
    let job = proto::SubmitJobRequest {
        job: Some(proto::Job {
            id: Some(0),
            nonce: None,
            program_verifying_key: input.clone(),
            input: input.clone(),
            contract_address: input.clone(),
            max_cycles: 100,
            result: input.clone(),
            zkvm_operator_address: input.clone(),
            zkvm_operator_signature: input,
            status: Some(JobStatus {
                status: JobStatusType::Unspecified as i32,
                failure_reason: None,
            }),
        }),
    };

    let serialized = serde_json::to_string(&job).expect("serialization failed");
    let expected_json = r#"
        {
            "job": {
                "id": 0,
                "nonce": null,
                "programVerifyingKey": "AAAB",
                "input": "AAAB",
                "contractAddress": "000001",
                "maxCycles": 100,
                "result": "AAAB",
                "zkvmOperatorAddress": "000001",
                "zkvmOperatorSignature": "AAAB",
                "status": {
                    "status": 0,
                    "failureReason": null
                }
            }
        }"#
    .replace(['\n', ' '], "");

    assert_eq!(serialized, expected_json);
}
