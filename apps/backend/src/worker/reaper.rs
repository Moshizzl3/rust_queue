use std::time::Duration;

use tokio_util::sync::CancellationToken;
use tracing::{info, warn, error};

use crate::repository::JobRepository;

/// The stale job reaper finds jobs stuck in 'running' state for too long
/// and resets them back to 'pending' so they can be retried.
///
/// If a worker crashes (OOM kill, hardware failure, hard SIGKILL), the job
/// it was processing stays in 'running' with a `locked_by` value pointing
/// to a worker that no longer exists. Without the reaper, these jobs are
/// lost forever.
///
/// How it works:
/// Every `check_interval`, it looks for jobs where:
///   status = 'running' AND started_at < NOW() - stale_threshold
///
/// These are jobs that have been running "too long" — likely orphaned.
/// It resets them to 'pending' so a healthy worker can pick them up.
///
/// The `stale_threshold` should be longer than your longest expected job
/// duration. If your slowest job takes 60s, set the threshold to 5 minutes.
/// Too short = you'll reset jobs that are still legitimately running.
/// Too long = orphaned jobs sit idle longer than necessary.
pub struct StaleJobReaper {
    repo: JobRepository,
    cancel_token: CancellationToken,
    check_interval: Duration,
    stale_threshold: Duration,
}

impl StaleJobReaper {
    pub fn new(
        repo: JobRepository,
        cancel_token: CancellationToken,
        check_interval: Duration,
        stale_threshold: Duration,
    ) -> Self {
        Self {
            repo,
            cancel_token,
            check_interval,
            stale_threshold,
        }
    }

    pub async fn run(&self) {
        info!(
            "Stale job reaper started, checking every {:?}, threshold {:?}",
            self.check_interval, self.stale_threshold
        );

        loop {
            if self.cancel_token.is_cancelled() {
                info!("Stale job reaper shutting down");
                break;
            }

            match self.reap_stale_jobs().await {
                Ok(count) if count > 0 => {
                    warn!(count, "Reset stale jobs back to pending");
                }
                Ok(_) => {} // no stale jobs, nothing to log
                Err(e) => {
                    error!(error = %e, "Error checking for stale jobs");
                }
            }

            tokio::select! {
                _ = tokio::time::sleep(self.check_interval) => {}
                _ = self.cancel_token.cancelled() => {
                    info!("Stale job reaper shutting down");
                    break;
                }
            }
        }
    }

    async fn reap_stale_jobs(&self) -> Result<i64, sqlx::Error> {
        let threshold_secs = self.stale_threshold.as_secs() as i64;
        self.repo.reset_stale_jobs(threshold_secs).await
    }
}

/// Spawn the reaper as a background task.
pub fn spawn_reaper(
    repo: JobRepository,
    cancel_token: CancellationToken,
    check_interval: Duration,
    stale_threshold: Duration,
) -> tokio::task::JoinHandle<()> {
    let reaper = StaleJobReaper::new(repo, cancel_token, check_interval, stale_threshold);
    tokio::spawn(async move {
        reaper.run().await;
    })
}
