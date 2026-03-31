use std::future::Future;
use std::pin::Pin;

/// The context passed to every job handler.
/// Contains the payload and metadata a handler might need.
pub struct JobContext {
    pub job_id: uuid::Uuid,
    pub job_type: String,
    pub payload: serde_json::Value,
    pub attempt: i32,
}

/// Trait for job handlers. Each job type (e.g. "send_email") gets its own
/// implementation.
pub trait JobHandler: Send + Sync {
    fn handle(
        &self,
        ctx: JobContext,
    ) -> Pin<Box<dyn Future<Output = Result<(), anyhow::Error>> + Send + '_>>;
}

/// Convenience: allow closures/functions to be used as handlers.
/// This means we can do:
///   registry.register("my_job", FnHandler(|ctx| Box::pin(async move { ... })));
pub struct FnHandler<F>(pub F);

impl<F, Fut> JobHandler for FnHandler<F>
where
    F: Fn(JobContext) -> Fut + Send + Sync,
    Fut: Future<Output = Result<(), anyhow::Error>> + Send + 'static,
{
    fn handle(
        &self,
        ctx: JobContext,
    ) -> Pin<Box<dyn Future<Output = Result<(), anyhow::Error>> + Send + '_>> {
        Box::pin(self.0(ctx))
    }
}
