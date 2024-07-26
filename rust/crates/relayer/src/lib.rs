use proto::Job;
use std::{
    collections::VecDeque,
    fmt::Debug,
    sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}},
    thread,
    time::Duration,
};

#[derive(Debug)]
pub enum Error {
    EthClientError,
}

pub trait EthClientI: Debug + Send + Sync {
    fn execute_callback(&self, job: &Job) -> Result<(), &str>;
}

#[derive(Debug)]
pub struct MockEthClient {
    pub times_called: Arc<Mutex<i32>>,
}

#[derive(Debug)]
pub struct EthClient;

impl EthClientI for MockEthClient {
    fn execute_callback(&self, job: &Job) -> Result<(), &str> {
        let mut times_called = self.times_called.lock().unwrap();
        *times_called += 1;
        println!("Executing callback for job: {}", job.id);
        Ok(())
    }
}

impl EthClientI for EthClient {
    fn execute_callback(&self, job: &Job) -> Result<(), &str> {
        println!("calling eth for job: {}", job.id);
        Ok(())
    }
}

#[derive(Debug)]
pub struct Relayer {
    pub eth_client: Arc<dyn EthClientI>,
    pub broadcast_queue: Arc<Mutex<VecDeque<Job>>>,
    pub stop_flag: Arc<AtomicBool>,
}

impl Relayer {
    pub fn new(eth_client: Arc<dyn EthClientI>, broadcast_queue: Arc<Mutex<VecDeque<Job>>>) -> Self {
        Relayer {
            eth_client,
            broadcast_queue,
            stop_flag: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn start(&self) -> Result<(), Error> {
        println!("Starting Relayer");

        while !self.stop_flag.load(Ordering::SeqCst) {
            let finished_job = {
                let mut queue = self.broadcast_queue.lock().unwrap();
                queue.pop_front()
            };

            if let Some(job) = finished_job {
                match self.eth_client.execute_callback(&job) {
                    Ok(_) => println!("Executed callback against job: {}", job.id),
                    Err(e) => {
                        println!("Error executing callback {}", e);
                        return Err(Error::EthClientError);
                    }
                }
            } else {
                thread::sleep(Duration::from_millis(10));
            }
        }

        println!("Relayer stopped");
        Ok(())
    }

    pub fn stop(&self) {
        self.stop_flag.store(true, Ordering::SeqCst);
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::sync::{Arc, Mutex};
    use tokio::time::{sleep, Duration};

    #[tokio::test]
    async fn test_relayer() {
        let mock_eth_client = Arc::new(MockEthClient {
            times_called: Arc::new(Mutex::new(0)),
        });

        let broadcast_queue = Arc::new(Mutex::new(VecDeque::new()));
        let relayer = Arc::new(Relayer::new(mock_eth_client.clone(), broadcast_queue.clone()));

        for i in 0..3 {
            let job = Job {
                id: i,
                program_verifying_key: vec![0x01, 0x02, 0x03],
                vm_type: 0,
                input: vec![0x04, 0x05, 0x06],
                contract_address: vec![0x07, 0x08, 0x09],
                max_cycles: 1000,
                result: vec![],
                zkvm_operator_address: vec![0x0A, 0x0B, 0x0C],
                zkvm_operator_signature: vec![0x0D, 0x0E, 0x0F],
                status: 1,
            };
            relayer.broadcast_queue.lock().unwrap().push_back(job);
        }

        let relayer_handle = {
            let relayer = relayer.clone();
            std::thread::spawn(move || {
                relayer.start().unwrap();
            })
        };

        sleep(Duration::from_secs(1)).await;
        relayer.stop();
        relayer_handle.join().unwrap();

        assert_eq!(*mock_eth_client.times_called.lock().unwrap(), 3);
        assert_eq!(relayer.broadcast_queue.lock().unwrap().len(), 0);
    }
}