//! Simple program to compare different `MemoryImage` initialization techniques.

use anyhow::Result;
use mock_consumer_methods::MOCK_CONSUMER_GUEST_ELF;
use risc0_binfmt::{MemoryImage, Program};
use risc0_zkvm::{ExecutorEnv, ExecutorImpl};
use risc0_zkvm_platform::{memory::GUEST_MAX_MEM, PAGE_SIZE};
use std::time::Instant;

fn main() -> Result<()> {
    // Initialize tracing. In order to view logs, run `RUST_LOG=info cargo run`
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::filter::EnvFilter::from_default_env())
        .init();

    let input: u32 = 15 * u32::pow(2, 27) + 1;

    println!("ELF len: {}", MOCK_CONSUMER_GUEST_ELF.len());

    let program = Program::load_elf(MOCK_CONSUMER_GUEST_ELF, GUEST_MAX_MEM as u32)?;
    // Measure time for MemoryImage::new
    let image_start = Instant::now();
    let image = MemoryImage::new(&program, PAGE_SIZE as u32)?;
    let image_duration = image_start.elapsed();
    println!("MemoryImage::new execution time: {:?}", image_duration);

    // Measure time for the default flow
    let start = Instant::now();
    {
        let env = ExecutorEnv::builder().write(&input)?.build()?;
        let _exec = ExecutorImpl::from_elf(env, MOCK_CONSUMER_GUEST_ELF)?;
        // exec.run()?;
    }
    let duration = start.elapsed();
    println!("from_elf execution time: {:?}", duration);

    // Measure time for the manual flow
    let start = Instant::now();
    {
        let env = ExecutorEnv::builder().write(&input)?.build()?;
        let _exec = ExecutorImpl::new(env, image)?;
        // let _session = exec.run()?;
    }
    let duration = start.elapsed();
    println!("Manual construction execution time: {:?}", duration);

    Ok(())
}
