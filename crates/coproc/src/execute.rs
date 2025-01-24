//! Actor for executing jobs on a per consumer basis.

use crate::{
    job_executor::ExecutorMsg,
    relayer::{RelayActorSpawner, RelayMsg},
};
use ivm_db::tables::Job;
use ivm_proto::RelayStrategy;
use std::collections::BTreeMap;
use tokio::{
    sync::{
        mpsc,
        mpsc::{Receiver, Sender},
        oneshot,
    },
    task::JoinSet,
};
use tracing::{debug, error, warn};

type JobNonce = u64;
type ExecutedJobs = BTreeMap<JobNonce, Job>;

/// The configuration for spawning job execution actors. A job execution actor will execute jobs as
/// they come in. As jobs finish executing, it will wait until the job with the next nonce finishes
/// and then send it to the relay actor.
#[derive(Debug, Clone)]
pub struct ExecutionActorSpawner {
    relay_actor_spawner: RelayActorSpawner,
    executor_tx: flume::Sender<ExecutorMsg>,
    channel_bound: usize,
}

impl ExecutionActorSpawner {
    /// Create a new instance of [Self].
    pub const fn new(
        executor_tx: flume::Sender<ExecutorMsg>,
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
    pub fn spawn(&self, nonce: u64) -> Sender<Job> {
        // Spawn a relay actor
        let relay_tx = self.relay_actor_spawner.spawn();
        let (tx, rx) = mpsc::channel(self.channel_bound);
        let actor = ExecutionActor::new(self.executor_tx.clone(), rx, relay_tx);

        tokio::spawn(async move { actor.start(nonce).await });

        tx
    }
}

struct ExecutionActor {
    executor_tx: flume::Sender<ExecutorMsg>,
    rx: Receiver<Job>,
    relay_tx: Sender<RelayMsg>,
}

impl ExecutionActor {
    const fn new(
        executor_tx: flume::Sender<ExecutorMsg>,
        rx: Receiver<Job>,
        relay_tx: Sender<RelayMsg>,
    ) -> Self {
        Self { executor_tx, rx, relay_tx }
    }

    /// The primary routine of the execution actor.
    ///
    /// We assume that the caller spawns exactly one job actor per consumer.
    async fn start(self, nonce: JobNonce) {
        let mut next_job_to_submit = nonce;
        let mut join_set = JoinSet::new();
        let mut completed_tasks = ExecutedJobs::new();
        let Self { mut rx, relay_tx, executor_tx } = self;

        loop {
            tokio::select! {
                // Handle a new job request by starting execution
                new_job = rx.recv() => {
                    match new_job {
                        Some(job) => {
                            let executor_tx2 = executor_tx.clone();
                            join_set.spawn(async move {
                                // Send the job to be executed
                                let (tx, executor_complete_rx) = oneshot::channel();
                                executor_tx2.send_async((job, tx)).await.expect("executor pool send failed");

                                // Return the executed job
                                executor_complete_rx.await
                            });
                        },
                        None => {
                            warn!("execute actor channel closed");
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
                                relay_tx.send(RelayMsg::Relay(job)).await.expect("relay actor send failed.");
                                continue;
                            }

                            if job.nonce == next_job_to_submit {
                                // If the completed job is the next job to relay, perform the relay
                                relay_tx.send(RelayMsg::Relay(job)).await.expect("relay actor send failed.");

                                next_job_to_submit += 1;
                                while let Some(next_job) = completed_tasks.remove(&next_job_to_submit) {
                                    // Relay any directly subsequent jobs that have been completed
                                    relay_tx.send(RelayMsg::Relay(next_job)).await.expect("relay actor send failed.");
                                    next_job_to_submit += 1;
                                }
                            } else {
                                // This is not the next job to relay, so just store it for later
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
                        None => if rx.is_closed() && rx.is_empty() {
                            debug!("exiting execution actor");
                            let _ = relay_tx.send(RelayMsg::Exit).await;
                            break;
                        } else {
                            // The channel is still open, so we continue to wait for new messages
                        },
                    }
                }
            }
        }
    }
}
