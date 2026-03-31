use std::collections::HashMap;
use std::sync::Arc;

use super::handler::JobHandler;

/// A registry that maps job type strings to handler implementations.
///
/// Arc<dyn JobHandler> because:
/// - Arc: shared across multiple worker tasks (each worker needs access)
/// - dyn JobHandler: different concrete types per job type (trait object)
///
/// The registry itself is wrapped in Arc when passed to workers,
/// so it's cheap to clone.
pub struct JobRegistry {
    handlers: HashMap<String, Arc<dyn JobHandler>>,
}

impl JobRegistry {
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
        }
    }

    /// Register a handler for a job type.
    /// If a handler was already registered for this type, it's replaced.
    pub fn register<H: JobHandler + 'static>(
        &mut self,
        job_type: impl Into<String>,
        handler: H,
    ) -> &mut Self {
        self.handlers.insert(job_type.into(), Arc::new(handler));
        self
    }

    /// Look up the handler for a job type.
    pub fn get(&self, job_type: &str) -> Option<Arc<dyn JobHandler>> {
        self.handlers.get(job_type).cloned()
    }

    /// List all registered job types. for health checks / debugging.
    pub fn registered_types(&self) -> Vec<&str> {
        self.handlers.keys().map(|s| s.as_str()).collect()
    }
}

impl Default for JobRegistry {
    fn default() -> Self {
        Self::new()
    }
}
