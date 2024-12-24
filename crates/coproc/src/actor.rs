//! Actor for processing jobs on a per consumer basis.

use std::collections::BTreeMap;

use flume::{Receiver, Sender};
use ivm_db::tables::Job;
use tokio::{sync::oneshot, task::JoinSet};
use crate::job_executor2::ExecutorMsg;

type JobNonce = u64;
type ExecutedJobs = BTreeMap<JobNonce, Job>;

/// The configuration for spawning job actors.
#[derive(Debug, Clone)]
pub struct JobActorSpawner {

    executor_tx: Sender<ExecutorMsg>,
}

impl JobActorSpawner {
    /// Create a new instance of [Self].
    pub fn new(
      executor_tx: Sender<ExecutorMsg>
    ) -> Self {
        Self { 
          executor_tx,
        }
    }

    /// Spawn a new job actor.
    /// 
    /// Caution: We assume that the caller spawns exactly one job actor per consumer.
    pub fn spawn(&self) -> Sender<Job> {
        // TODO: add bound
        let (actor_tx, actor_rx) = flume::bounded(4094);

        let executor_tx2 = self.executor_tx.clone();
        tokio::spawn(async move { Self::start_actor( executor_tx2, actor_rx).await });

        actor_tx
    }

    /// Internal method to start a new actor. This is the primary routine of the actor.
    ///
    /// We assume that the caller spawns exactly one job actor per consumer.
    async fn start_actor(executor_tx: Sender<ExecutorMsg>, actor_rx: Receiver<Job>) {
        let mut join_set = JoinSet::new();

        // Use a priority queue.
        let mut completed_tasks = ExecutedJobs::new();

        // TODO: robust logic to initialize job
        let mut next_job_to_submit: JobNonce = 1;

        loop {
            tokio::select! {
                // Handle a new job request by starting execution
                new_job = actor_rx.recv_async() => {
                    match new_job {
                        Ok(job) => {
                            println!("received {:?}", job.nonce);
                            let executor_tx2 = executor_tx.clone();
                            join_set.spawn(async move {
                                // Send the job to be executed
                                let (tx, executor_complete_rx) = oneshot::channel();
                                executor_tx2.send_async((job, tx)).await.expect("todo");

                                // Return the executed job
                                executor_complete_rx.await
                            });
                        },
                        Err(_e) => continue, // TODO
                    }
                }
                // As jobs complete
                completed = join_set.join_next() => {
                    match completed {
                        Some(Ok(Ok(job))) => {
                            // If the completed job is the next job to relay, perform the relay
                            if job.nonce == next_job_to_submit {
                                let mut jobs_to_relay = Vec::new();
                                jobs_to_relay.push(job);

                                next_job_to_submit += 1;
                                while let Some(next_job) = completed_tasks.remove(&next_job_to_submit) {
                                  jobs_to_relay.push(next_job);
                                  next_job_to_submit += 1;
                                }

                                // TODO: relaying
                            } else {
                                completed_tasks.insert(job.nonce as u64, job);
                            }
                        },
                        Some(Ok(Err(e)))  => {
                            println!("job error: {:?}", e);
                        },
                        Some(Err(e)) => {
                            println!("fatal error: {:?}", e);
                        }
                        // The join set is empty so we check if the channle is still open
                        None => if actor_rx.is_empty() && actor_rx.is_disconnected() {
                            println!("All done ending task");
                            break;
                        } else {
                            continue;
                        },
                    }
                }
            }
        }
    }
}

// #[tokio::main]
// async fn main() {
//     let (tx, mut rx) = tokio::sync::mpsc::channel(10);
//     for i in (1..=10).rev() {
//         tx.send(i).await.unwrap();
//     }
//     drop(tx);
// }
