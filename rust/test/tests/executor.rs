use integration::{Clients, Integration};
use proto::{ExecuteRequest, VerifiedInputs};

#[tokio::test]
#[ignore]
async fn executor_fails_with_bad_inputs() {
    async fn test(mut clients: Clients) {
        let request = ExecuteRequest {
            program_elf: vec![],
            inputs: Some(VerifiedInputs {
                program_verifying_key: vec![],
                program_input: vec![],
                max_cycles: 32 * 1000,
            }),
        };

        let result = clients.executor.execute(request).await;

        assert!(result.is_err());
    }

    Integration::run(test).await
}
