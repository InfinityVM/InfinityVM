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
use tracing::{debug, error, info, warn};

type JobNonce = u64;
type ExecutedJobs = BTreeMap<JobNonce, Job>;

/// The configuration for spawning job actors.
#[derive(Debug, Clone)]
pub struct ExecutionActorSpawner {
    relay_actor_spawner: RelayActorSpawner,
    executor_tx: Sender<ExecutorMsg>,
}

impl ExecutionActorSpawner {
    /// Create a new instance of [Self].
    pub const fn new(
        executor_tx: Sender<ExecutorMsg>,
        relay_actor_spawner: RelayActorSpawner,
    ) -> Self {
        Self { executor_tx, relay_actor_spawner }
    }

    /// Spawn a new job actor.
    ///
    /// Note that this will also spawn a corresponding
    ///
    /// Caution: We assume that the caller spawns exactly one job actor per consumer.
    pub fn spawn(&self) -> Sender<Job> {
        // Spawn a relay actor
        let relay_tx = self.relay_actor_spawner.spawn();

        // TODO: add bound
        let (tx, rx) = flume::bounded(4094);

        let executor_tx2 = self.executor_tx.clone();
        tokio::spawn(async move { start_actor(executor_tx2, rx, relay_tx).await });

        tx
    }
}

/// The primary routine of the execution actor.
///
/// We assume that the caller spawns exactly one job actor per consumer.
async fn start_actor(
    executor_tx: Sender<ExecutorMsg>,
    rx: Receiver<Job>,
    relay_tx: Sender<RelayMsg>,
) {
    let mut join_set = JoinSet::new();

    // Use a priority queue.
    let mut completed_tasks = ExecutedJobs::new();

    // TODO: robust logic to initialize job
    let mut next_job_to_submit: JobNonce = 0;

    loop {
        tokio::select! {
            // Handle a new job request by starting execution
            new_job = rx.recv_async() => {
                match new_job {
                    Ok(job) => {
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
            // As jobs complete, relay them as determined by their nonce
            completed = join_set.join_next() => {
                match completed {
                    Some(Ok(Ok(job))) => {
                        info!(
                            job.nonce,
                            next_job_to_submit,
                            "completed, about to handle"
                        );
                        // Short circuit ordering logic and relay immediately if this job is
                        // not ordered relay.
                        if job.relay_strategy == RelayStrategy::Unordered {
                            relay_tx.send_async(RelayMsg::Relay(job)).await.expect("relay actor send failed.");
                            continue;
                        }

                        // If the completed job is the next job to relay, perform the relay
                        if job.nonce == next_job_to_submit {
                            relay_tx.send_async(RelayMsg::Relay(job)).await.expect("relay actor send failed.");
                            info!(
                                next_job_to_submit,
                                "completed, sent as matching next nonce"
                            );

                            next_job_to_submit += 1;
                            while let Some(next_job) = completed_tasks.remove(&next_job_to_submit) {
                                info!(
                                    next_job_to_submit,
                                    "completed, sent as backlogged job"
                                );
                                relay_tx.send_async(RelayMsg::Relay(next_job)).await.expect("relay actor send failed.");
                                next_job_to_submit += 1;
                            }
                        } else {
                            info!(
                                next_job_to_submit,
                                "completed, inserted into completed tasks"
                            );
                            completed_tasks.insert(job.nonce, job);
                        }
                    },
                    Some(Ok(Err(error)))  => {
                        warn!(?error, "execution error");
                    },
                    Some(Err(error)) => {
                        error!(?error, "fatal error exiting execution actor for");
                    }
                    // The join set is empty so we check if the channel is still open
                    // TODO: do we want to end just on the subscriber being empty? Seems like it would never actually become disconnected
                    None => if rx.is_empty() && rx.is_disconnected() {
                        debug!("exiting execution actor");
                        let _ = relay_tx.send_async(RelayMsg::Exit).await;
                        break;
                    } else {
                        continue;
                    },
                }
            }
        }
    }
}
