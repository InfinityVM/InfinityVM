//! Binding to ZKVM programs.

include!(concat!(env!("OUT_DIR"), "/methods.rs"));

#[cfg(test)]
mod tests {
    // use clob_core::{StfInput, StfOutput};
    // use risc0_zkvm::{Executor, ExecutorEnv, LocalProver};

    // // TODO: fix this
    // #[test]
    // fn executes_program() {
    //     let zkvm_executor = LocalProver::new("locals only");

    //     let zkvm_input = borsh::to_vec(&(&request, &state)).expect("borsh serialize works.
    // qed.");     let env = ExecutorEnv::builder()
    //         .write::<u32>(&(zkvm_input.len() as u32))
    //         .expect("u32 is always writable")
    //         .write_slice(&zkvm_input)
    //         .build()
    //         .unwrap();

    //     let execute_info = zkvm_executor.execute(env, super::CLOB_ELF).unwrap();

    //     let (z_response, z_post_state): clob_core::StfOutput =
    //         borsh::from_slice(&execute_info.journal.bytes).expect("todo");
    // }
}
