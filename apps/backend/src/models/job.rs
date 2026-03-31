use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

// ── Status enum ────────────────────────────────────────────────────────────
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema, sqlx::Type)]
#[sqlx(type_name = "job_status", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum JobStatus {
    Pending,
    Running,
    Completed,
    Dead,
    Cancelled,
}

impl std::fmt::Display for JobStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JobStatus::Pending => write!(f, "pending"),
            JobStatus::Running => write!(f, "running"),
            JobStatus::Completed => write!(f, "completed"),
            JobStatus::Dead => write!(f, "dead"),
            JobStatus::Cancelled => write!(f, "cancelled"),
        }
    }
}

// ── Job entity ─────────────────────────────────────────────────────────────
#[derive(Debug, Clone, FromRow, Serialize, ToSchema)]
pub struct Job {
    pub id: Uuid,
    pub job_type: String,
    pub payload: serde_json::Value,

    pub status: JobStatus,
    pub priority: i16,

    pub attempt: i32,
    pub max_retries: i32,
    pub scheduled_at: DateTime<Utc>,

    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub locked_by: Option<String>,
    pub last_error: Option<String>,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ── API request types ──────────────────────────────────────────────────────

#[derive(Debug, Deserialize, ToSchema, Validate)]
pub struct CreateJobRequest {
    /// The job type identifier, e.g. "send_email", "generate_report"
    #[validate(length(min = 1, max = 255, message = "job_type must be 1-255 characters"))]
    #[schema(example = "send_email")]
    pub job_type: String,

    /// Arbitrary JSON payload passed to the job handler
    #[schema(example = json!({"to": "user@example.com", "template": "welcome"}))]
    #[serde(default = "default_payload")]
    pub payload: serde_json::Value,

    /// Priority: 0 (highest) to 10 (lowest). Default 5.
    #[validate(range(min = 0, max = 10, message = "priority must be 0-10"))]
    #[schema(example = 5)]
    #[serde(default = "default_priority")]
    pub priority: Option<i16>,

    /// Maximum retry attempts. Default 3.
    #[validate(range(min = 0, max = 20, message = "max_retries must be 0-20"))]
    #[schema(example = 3)]
    pub max_retries: Option<i32>,

    /// Don't run before this time. Omit or null for "run immediately".
    pub scheduled_at: Option<DateTime<Utc>>,
}

fn default_payload() -> serde_json::Value {
    serde_json::Value::Object(serde_json::Map::new())
}

fn default_priority() -> Option<i16> {
    Some(5)
}

// ── API response types ─────────────────────────────────────────────────────
#[derive(Debug, Serialize, ToSchema)]
pub struct JobResponse {
    pub id: Uuid,
    pub job_type: String,
    pub payload: serde_json::Value,
    pub status: JobStatus,
    pub priority: i16,
    pub attempt: i32,
    pub max_retries: i32,
    pub scheduled_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl From<Job> for JobResponse {
    fn from(job: Job) -> Self {
        Self {
            id: job.id,
            job_type: job.job_type,
            payload: job.payload,
            status: job.status,
            priority: job.priority,
            attempt: job.attempt,
            max_retries: job.max_retries,
            scheduled_at: job.scheduled_at,
            completed_at: job.completed_at,
            last_error: job.last_error,
            created_at: job.created_at,
        }
    }
}

/// Admin/dashboard response with full internal details
#[derive(Debug, Serialize, ToSchema)]
pub struct JobDetailResponse {
    pub id: Uuid,
    pub job_type: String,
    pub payload: serde_json::Value,
    pub status: JobStatus,
    pub priority: i16,
    pub attempt: i32,
    pub max_retries: i32,
    pub scheduled_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub locked_by: Option<String>,
    pub last_error: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<Job> for JobDetailResponse {
    fn from(job: Job) -> Self {
        Self {
            id: job.id,
            job_type: job.job_type,
            payload: job.payload,
            status: job.status,
            priority: job.priority,
            attempt: job.attempt,
            max_retries: job.max_retries,
            scheduled_at: job.scheduled_at,
            started_at: job.started_at,
            completed_at: job.completed_at,
            locked_by: job.locked_by,
            last_error: job.last_error,
            created_at: job.created_at,
            updated_at: job.updated_at,
        }
    }
}

/// Summary stats for the dashboard
#[derive(Debug, Serialize, FromRow, ToSchema)]
pub struct JobStats {
    pub pending: i64,
    pub running: i64,
    pub completed: i64,
    pub dead: i64,
    pub cancelled: i64,
}
