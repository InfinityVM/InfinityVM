pub(crate) fn anvil_ip() -> String {
    env::var("ANVIL_IP").unwrap_or_else(|_| "127.0.0.1".to_string())
}

pub(crate) fn anvil_port() -> String {
    env::var("ANVIL_PORT").unwrap_or_else(|_| "8545".to_string())
}

pub(crate) fn coprocessor_gateway_ip() -> String {
    env::var("COPROCESSOR_GATEWAY_IP").unwrap_or_else(|_| "127.0.0.1".to_string())
}

pub(crate) fn coprocessor_gateway_port() -> String {
    env::var("COPROCESSOR_GATEWAY_PORT").unwrap_or_else(|_| "8080".to_string())
}

pub(crate) fn max_cycles() -> u64 {
    match env::var("MAX_CYCLES") {
        Ok(max_cycles) => max_cycles.parse().unwrap_or(1000000),
        Err(_) => 1000000, // Default value if MAX_CYCLES is not set
    }
}

pub(crate) fn consumer_addr() -> String {
    match env::var("CONSUMER_ADDR") {
        Ok(consumer_addr) => consumer_addr,
        Err(_) => {
            // If env var is not set, try to get the consumer address from the deploy info
            let deploy_info = get_default_deploy_info();
            match deploy_info {
                Ok(deploy_info) => deploy_info.mock_consumer.to_string(),
                Err(_) => "0xbdEd0D2bf404bdcBa897a74E6657f1f12e5C6fb6".to_string(), /* Default consumer address */
            }
        }
    }
}

pub(crate) fn should_wait_until_job_completed() -> bool {
    match env::var("WAIT_UNTIL_JOB_COMPLETED") {
        Ok(enabled) => enabled.to_lowercase() == "true",
        Err(_) => true,
    }
}

// Default to 10 users if not set or invalid
pub(crate) fn num_users() -> usize {
    env::var("NUM_USERS").ok().and_then(|v| v.parse().ok()).unwrap_or(10)
}

pub(crate) fn report_file_name() -> String {
    env::var("REPORT_FILE_NAME").ok().unwrap_or_else(|| "report.html".to_string())
}

// Default to 10 seconds if not set or invalid
fn startup_time() -> usize {
    env::var("STARTUP_TIME").ok().and_then(|v| v.parse().ok()).unwrap_or(10)
}

// Default to 20 seconds if not set or invalid
fn run_time() -> usize {
    env::var("RUN_TIME").ok().and_then(|v| v.parse().ok()).unwrap_or(20)
}
