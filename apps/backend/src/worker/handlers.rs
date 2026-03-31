//! Demo job handlers for testing and dashboard demonstration.
//!
//! In a real system, these would be replaced with actual implementations
//! (sending emails via SES, generating PDFs, calling APIs, etc).
//! These simulate realistic behavior: variable durations and failure rates.

use rand::Rng;
use tracing::info;

use super::handler::{FnHandler, JobContext};
use super::registry::JobRegistry;

/// Register all demo handlers into a registry.
pub fn register_demo_handlers(registry: &mut JobRegistry) {
    registry
        .register("fast_task", FnHandler(handle_fast_task))
        .register("slow_task", FnHandler(handle_slow_task))
        .register("flaky_task", FnHandler(handle_flaky_task))
        .register("critical_report", FnHandler(handle_critical_report));
}

/// Fast task: 1-3 seconds, almost never fails (~5% failure rate).
async fn handle_fast_task(ctx: JobContext) -> Result<(), anyhow::Error> {
    // Do ALL random work before any .await — ThreadRng is !Send
    // and can't be held across await points in a multi-threaded runtime.
    let (duration, should_fail) = {
        let mut rng = rand::rng();
        (rng.random_range(1..=3), rng.random_range(0..100) < 5)
    }; // rng is dropped here, before the .await

    info!(job_id = %ctx.job_id, "Fast task: working for {}s", duration);
    tokio::time::sleep(std::time::Duration::from_secs(duration)).await;

    if should_fail {
        anyhow::bail!("Fast task failed: simulated transient error");
    }

    Ok(())
}

/// Slow task: 5-15 seconds, occasionally fails (~15% failure rate).
async fn handle_slow_task(ctx: JobContext) -> Result<(), anyhow::Error> {
    let (duration, should_fail) = {
        let mut rng = rand::rng();
        (rng.random_range(5..=15), rng.random_range(0..100) < 15)
    };

    info!(job_id = %ctx.job_id, "Slow task: working for {}s", duration);
    tokio::time::sleep(std::time::Duration::from_secs(duration)).await;

    if should_fail {
        anyhow::bail!("Slow task failed: simulated processing error");
    }

    Ok(())
}

/// Flaky task: 2-5 seconds, fails ~50% of the time.
/// for demonstrating retry logic and exponential backoff.
async fn handle_flaky_task(ctx: JobContext) -> Result<(), anyhow::Error> {
    let (duration, should_fail) = {
        let mut rng = rand::rng();
        (rng.random_range(2..=5), rng.random_range(0..100) < 50)
    };

    info!(
        job_id = %ctx.job_id,
        attempt = ctx.attempt,
        "Flaky task: attempt {}, working for {}s",
        ctx.attempt,
        duration
    );
    tokio::time::sleep(std::time::Duration::from_secs(duration)).await;

    if should_fail {
        anyhow::bail!(
            "Flaky task failed on attempt {}: simulated intermittent failure",
            ctx.attempt
        );
    }

    Ok(())
}

/// Critical report: 3-8 seconds, low failure rate (~10%).
async fn handle_critical_report(ctx: JobContext) -> Result<(), anyhow::Error> {
    let (duration, should_fail) = {
        let mut rng = rand::rng();
        (rng.random_range(3..=8), rng.random_range(0..100) < 10)
    };

    info!(job_id = %ctx.job_id, "Critical report: working for {}s", duration);
    tokio::time::sleep(std::time::Duration::from_secs(duration)).await;

    if should_fail {
        anyhow::bail!("Critical report failed: simulated downstream service error");
    }

    Ok(())
}
