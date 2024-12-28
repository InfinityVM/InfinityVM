//! Actor for processing jobs on a per consumer basis.

use crate::{
    job_executor::ExecutorMsg,
    relayer::{RelayActorSpawner, RelayMsg},
};
use flume::{Receiver, Sender};
use ivm_db::tables::Job;
use ivm_proto::RelayStrategy;
use std::collections::BTreeMap;
use tokio::{sync::oneshot, task::JoinSet};
use tracing::{debug, error, warn};

type JobNonce = u64;
type ExecutedJobs = BTreeMap<JobNonce, Job>;

/// The configuration for spawning job actors.
#[derive(Debug, Clone)]
pub struct ExecutionActorSpawner {
    relay_actor_spawner: RelayActorSpawner,
    executor_tx: Sender<ExecutorMsg>,
    channel_bound: usize,
}

impl ExecutionActorSpawner {
    /// Create a new instance of [Self].
    pub const fn new(
        executor_tx: Sender<ExecutorMsg>,
        relay_actor_spawner: RelayActorSpawner,
        channel_bound: usize,
    ) -> Self {
        Self { executor_tx, relay_actor_spawner, channel_bound }
    }

    /// Spawn a new job actor.
    ///
    /// Note that this will also spawn a corresponding relay actor.
    ///
    /// Caution: we assume that the caller spawns exactly one job actor per consumer.
    pub fn spawn(&self) -> Sender<Job> {
        // Spawn a relay actor
        let relay_tx = self.relay_actor_spawner.spawn();
        let (tx, rx) = flume::bounded(self.channel_bound);
        let actor = ExecutionActor::new(self.executor_tx.clone(), rx, relay_tx);

        tokio::spawn(async move { actor.start().await });

        tx
    }
}

struct ExecutionActor {
    executor_tx: Sender<ExecutorMsg>,
    rx: Receiver<Job>,
    relay_tx: Sender<RelayMsg>,
}

impl ExecutionActor {
    const fn new(
        executor_tx: Sender<ExecutorMsg>,
        rx: Receiver<Job>,
        relay_tx: Sender<RelayMsg>,
    ) -> Self {
        Self { executor_tx, rx, relay_tx }
    }

    /// The primary routine of the execution actor.
    ///
    /// We assume that the caller spawns exactly one job actor per consumer.
    async fn start(self) {
        let mut join_set = JoinSet::new();

        // Use a priority queue.
        let mut completed_tasks = ExecutedJobs::new();

        // TODO: robust logic to initialize job
        let mut next_job_to_submit: JobNonce = 0;

        loop {
            tokio::select! {
                // Handle a new job request by starting execution
                new_job = self.rx.recv_async() => {
                    match new_job {
                        Ok(job) => {
                            let executor_tx2 = self.executor_tx.clone();
                            join_set.spawn(async move {
                                // Send the job to be executed
                                let (tx, executor_complete_rx) = oneshot::channel();
                                executor_tx2.send_async((job, tx)).await.expect("executor pool send failed");

                                // Return the executed job
                                executor_complete_rx.await
                            });
                        },
                        Err(_e) => {
                            warn!("execute actor receiver error, exiting");
                            break
                        },
                    }
                }
                // As jobs complete, relay them as determined by their nonce
                completed = join_set.join_next() => {
                    match completed {
                        Some(Ok(Ok(job))) => {
                            // Short circuit ordering logic and relay immediately if this job is
                            // not ordered relay.
                            if job.relay_strategy == RelayStrategy::Unordered {
                                self.relay_tx.send_async(RelayMsg::Relay(job)).await.expect("relay actor send failed.");
                                continue;
                            }

                            // If the completed job is the next job to relay, perform the relay
                            if job.nonce == next_job_to_submit {
                                self.relay_tx.send_async(RelayMsg::Relay(job)).await.expect("relay actor send failed.");

                                next_job_to_submit += 1;
                                while let Some(next_job) = completed_tasks.remove(&next_job_to_submit) {
                                    self.relay_tx.send_async(RelayMsg::Relay(next_job)).await.expect("relay actor send failed.");
                                    next_job_to_submit += 1;
                                }
                            } else {
                                completed_tasks.insert(job.nonce, job);
                            }
                        },
                        Some(Ok(Err(error)))  => {
                            warn!(?error, "execution error");
                        },
                        Some(Err(error)) => {
                            error!(?error, "fatal error, exiting execution actor");
                            break;
                        }
                        // The join set is empty so we check if the channel is still open
                        None => if self.rx.is_empty() && self.rx.is_disconnected() {
                            debug!("exiting execution actor");
                            let _ = self.relay_tx.send_async(RelayMsg::Exit).await;
                            break;
                        } else {
                            // The channel is still open, so we continue to wait for new messages
                            continue;
                        },
                    }
                }
            }
        }
    }
}
