use clob_programs::CLOB_ELF;
use e2e::{Args, E2EBuilder};
use proto::SubmitProgramRequest;
use proto::VmType;
use risc0_binfmt::compute_image_id;

fn program_id() -> Vec<u8> {
    compute_image_id(CLOB_ELF).unwrap().as_bytes().to_vec()
}

#[tokio::test]
#[ignore]
async fn state_job_submissions_clob_consumer() {
    async fn test(mut args: Args) {
        let anvil = args.anvil;
        let clob = args.clob_consumer.unwrap();
        let program_id = program_id();

        // Seed coprocessor-node with ELF
        let submit_program_request =
            SubmitProgramRequest { program_elf: CLOB_ELF.to_vec(), vm_type: VmType::Risc0.into() };
        let submit_program_response = args
            .coprocessor_node
            .submit_program(submit_program_request)
            .await
            .unwrap()
            .into_inner();
        assert_eq!(submit_program_response.program_id, program_id);

        
    }
    E2EBuilder::new().clob().build(test).await;
}
