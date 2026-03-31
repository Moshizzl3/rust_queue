use std::sync::Arc;

use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};

use crate::repository::JobRepository;
use super::handler::JobContext;
use super::registry::JobRegistry;

/// The background worker that polls for and executes jobs.
pub struct Worker {
    id: String,
    repo: JobRepository,
    registry: Arc<JobRegistry>,
    cancel_token: CancellationToken,
    poll_interval: std::time::Duration,
}

impl Worker {
    pub fn new(
        id: String,
        repo: JobRepository,
        registry: Arc<JobRegistry>,
        cancel_token: CancellationToken,
        poll_interval: std::time::Duration,
    ) -> Self {
        Self {
            id,
            repo,
            registry,
            cancel_token,
            poll_interval,
        }
    }

    /// Run the worker loop. This blocks until the cancellation token is triggered.
    ///
    /// The loop structure:
    /// 1. Try to acquire a job
    /// 2. If we got one, execute it
    /// 3. If no job available, sleep for poll_interval
    /// 4. Check cancellation between each cycle
    ///
    /// Important: we check cancellation BETWEEN jobs, not during.
    /// This means if a worker is mid-execution when SIGTERM arrives,
    /// it finishes the current job before stopping for graceful shutdown.
    pub async fn run(&self) {
        info!(worker_id = %self.id, "Worker started, polling every {:?}", self.poll_interval);

        loop {
            // Check if we've been told to shut down
            if self.cancel_token.is_cancelled() {
                info!(worker_id = %self.id, "Shutdown signal received, stopping worker");
                break;
            }

            match self.poll_and_execute().await {
                Ok(true) => {
                    // Processed a job — immediately poll for more.
                    // No sleep here: if there's a backlog, we want to
                    // drain it as fast as possible.
                    continue;
                }
                Ok(false) => {
                    // No jobs available — sleep before polling again.
                    // Use tokio::select! so we wake up immediately on
                    // cancellation instead of waiting out the full interval.
                    tokio::select! {
                        _ = tokio::time::sleep(self.poll_interval) => {}
                        _ = self.cancel_token.cancelled() => {
                            info!(worker_id = %self.id, "Shutdown signal during sleep, stopping worker");
                            break;
                        }
                    }
                }
                Err(e) => {
                    // Database error during polling — log and back off
                    // to avoid hammering a potentially down database.
                    error!(worker_id = %self.id, error = %e, "Error polling for jobs");
                    tokio::select! {
                        _ = tokio::time::sleep(self.poll_interval * 2) => {}
                        _ = self.cancel_token.cancelled() => {
                            break;
                        }
                    }
                }
            }
        }

        info!(worker_id = %self.id, "Worker stopped");
    }

    /// Try to acquire and execute a single job.
    /// Returns Ok(true) if a job was processed, Ok(false) if none available.
    async fn poll_and_execute(&self) -> Result<bool, sqlx::Error> {
        let job = match self.repo.acquire_next(&self.id).await? {
            Some(job) => job,
            None => return Ok(false),
        };

        let job_id = job.id;
        let job_type = job.job_type.clone();

        info!(
            worker_id = %self.id,
            job_id = %job_id,
            job_type = %job_type,
            attempt = %job.attempt,
            "Executing job"
        );

        // Look up the handler for this job type
        let handler = match self.registry.get(&job_type) {
            Some(h) => h,
            None => {
                // No handler registered for this job type.
                // This is a configuration error, not a transient failure,
                // so we fail the job with a clear message.
                warn!(
                    worker_id = %self.id,
                    job_id = %job_id,
                    job_type = %job_type,
                    "No handler registered for job type"
                );
                let _ = self
                    .repo
                    .fail(
                        job_id,
                        &format!("No handler registered for job type '{}'", job_type),
                    )
                    .await;
                return Ok(true);
            }
        };

        // Build the context and execute
        let ctx = JobContext {
            job_id,
            job_type: job_type.clone(),
            payload: job.payload.clone(),
            attempt: job.attempt,
        };

        match handler.handle(ctx).await {
            Ok(()) => {
                info!(
                    worker_id = %self.id,
                    job_id = %job_id,
                    job_type = %job_type,
                    "Job completed successfully"
                );
                let _ = self.repo.complete(job_id).await;
            }
            Err(e) => {
                warn!(
                    worker_id = %self.id,
                    job_id = %job_id,
                    job_type = %job_type,
                    error = %e,
                    "Job failed"
                );
                let _ = self.repo.fail(job_id, &e.to_string()).await;
            }
        }

        Ok(true)
    }
}

/// Spawn multiple worker tasks and return their join handles.
///
/// This is the main entry point called from main.rs.
pub fn spawn_workers(
    count: usize,
    repo: JobRepository,
    registry: Arc<JobRegistry>,
    cancel_token: CancellationToken,
    poll_interval: std::time::Duration,
) -> Vec<tokio::task::JoinHandle<()>> {
    (0..count)
        .map(|i| {
            let worker = Worker::new(
                format!("worker-{}", i),
                repo.clone(),
                registry.clone(),
                cancel_token.clone(),
                poll_interval,
            );
            tokio::spawn(async move {
                worker.run().await;
            })
        })
        .collect()
}
