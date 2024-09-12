use proto::{JobResult, JobStatus, JobStatusType};

#[test]
#[ignore]
fn serde_json_test() {
    let input = vec![0, 0, 1];
    let job_result = JobResult {
        id: input.clone(),
        nonce: 1,
        max_cycles: 100,
        consumer_address: input.clone(),
        program_id: input.clone(),
        onchain_input: input.clone(),
        request_signature: input.clone(),
        result_with_metadata: input.clone(),
        zkvm_operator_signature: input,
        status: Some(JobStatus {
            status: JobStatusType::Pending as i32,
            failure_reason: None,
            retries: 0,
        }),
    };

    let serialized = serde_json::to_string(&job_result).expect("serialization failed");
    let expected_json = r#"
        {
            "id": "000001",
            "nonce": 1,
            "maxCycles": 100,
            "consumerAddress": "000001",
            "programId": "000001",
            "onchainInput": "000001",
            "requestSignature": "000001",
            "resultWithMetadata": "000001",
            "zkvmOperatorSignature": "000001",
            "status": {
                "status": 1,
                "failureReason": null,
                "retries": 0
            }
        }"#
    .replace(['\n', ' '], "");

    assert_eq!(serialized, expected_json);
}
