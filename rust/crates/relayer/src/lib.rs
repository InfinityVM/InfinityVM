use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use std::fmt::Debug;
use proto::{Job};

#[derive(Debug)]
enum Error {
    EthClientError,
}

pub trait EthClient: Debug { // mock this
    fn execute_callback(&self,  job: &Job)-> Result<(), &str>;
}

#[derive(Debug)]
pub struct MockEthClient{
    pub times_called: Arc<Mutex<i32>>,
}

impl EthClient for MockEthClient{
    fn execute_callback(&self, job: &Job) -> Result<(), &str> {
        println!("Executing callback for job: {}", job.id);
        Ok(())
    }
}

impl MockEthClient {
    fn execute_callback(&mut self, job: &Job) -> Result<(), &str> {
        let mut times_called = self.times_called.lock().unwrap();
        *times_called += 1;
        Ok(())
    }
}

#[derive(Debug)]
pub struct Relayer {
    pub eth_client: Box<dyn EthClient>,
    pub broadcast_queue: Arc<Mutex<VecDeque<Job>>>,
}

impl Relayer {
    pub fn new(eth_client: Box<dyn EthClient>, broadcast_queue: Arc<Mutex<VecDeque<Job>>>) -> Self {
        Relayer {
            eth_client,
            broadcast_queue
        }
    }
    pub fn start(&mut self) -> Result<(), Error>{
        println!("Starting Relayer");

       loop {
           let finished_job = self.broadcast_queue.lock().unwrap().pop_front();

           if let Some(job) = finished_job {
               match self.eth_client.execute_callback(&job) {
                   Ok(_) => println!("Executed callback against job: {}", job.id),
                   Err(e) => {
                       println!("Error executing callback {}", e);
                       return Err(Error::EthClientError)
                   }
               }
           } else {
               println!("No jobs to process");
               thread::sleep(Duration::from_millis(100));
           }
       }
    }

    pub fn stop(self) -> Result<(), Error> {
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::sync::{Arc, Mutex};
    use tokio::time::{sleep, Duration};


    #[derive(Debug)]
    pub struct MockEthClient{
        pub times_called: Arc<Mutex<i32>>,
    }

    #[tokio::test]
    async fn test_relayer() {
        let mock_eth_client = Arc::new(MockEthClient{
            times_called: Arc::new(Mutex::new(0)),
        });
        let broadcast_queue = Arc::new(Mutex::new(VecDeque::new()));
        let mut relayer = Relayer::new(mock_eth_client.clone(), broadcast_queue.clone());

        for i in 0..3 {
            let job = Job {
              id: i,
                program_verifying_key: vec![0x01, 0x02, 0x03],
                vm_type:0,
                input: vec![0x04, 0x05, 0x06],
                contract_address: vec![0x07, 0x08, 0x09],
                max_cycles: 1000,
                result: vec![],
                zkvm_operator_address: vec![0x0A, 0x0B, 0x0C],
                zkvm_operator_signature: vec![0x0D, 0x0E, 0x0F],
                status: 1,
            };
            let mut queue = relayer.broadcast_queue.lock().unwrap().push_back(job);
        }

        let relayer_thread = {
            tokio::spawn(async move {
                relayer.start().unwrap();
            })
        };

        sleep(Duration::from_secs(1)).await;
        relayer.stop();
        relayer_thread.await.unwrap();

        assert_eq!(*mock_eth_client.times_called.lock().unwrap(), 3);
        assert_eq!(relayer.broadcast_queue.lock().unwrap().len(), 0);
    }
}