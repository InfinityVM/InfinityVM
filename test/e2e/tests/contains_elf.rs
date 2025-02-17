//! Tests for contains elf endpoint.

use e2e::{Args, E2E};
use ivm_proto::{ContainsProgramRequest, SubmitProgramRequest, VmType};
use mock_consumer_programs::{MOCK_CONSUMER_ELF, MOCK_CONSUMER_PROGRAM_ID};

#[ignore]
#[tokio::test(flavor = "multi_thread")]
async fn contains_elf() {
    async fn test(mut args: Args) {
        let program_id = MOCK_CONSUMER_PROGRAM_ID;

        let contains_request = ContainsProgramRequest { program_id: program_id.to_vec() };
        let contains = args
            .coprocessor_node
            .contains_program(contains_request.clone())
            .await
            .unwrap()
            .into_inner()
            .contains;
        assert!(!contains);

        // Seed coprocessor-node with ELF
        let submit_program_request = SubmitProgramRequest {
            program_elf: MOCK_CONSUMER_ELF.to_vec(),
            vm_type: VmType::Sp1.into(),
            program_id: program_id.to_vec(),
        };
        let submit_program_response = args
            .coprocessor_node
            .submit_program(submit_program_request)
            .await
            .unwrap()
            .into_inner();
        assert_eq!(submit_program_response.program_id, program_id);

        let contains = args
            .coprocessor_node
            .contains_program(contains_request.clone())
            .await
            .unwrap()
            .into_inner()
            .contains;
        assert!(contains);
    }

    E2E::new().mock_consumer().run(test).await;
}
