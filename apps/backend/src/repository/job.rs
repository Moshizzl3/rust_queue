use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::job::{CreateJobRequest, Job, JobStats, JobStatus};
use crate::models::pagination::{PagedData, PaginationParams};

#[derive(Clone)]
pub struct JobRepository {
    pool: PgPool,
}

impl JobRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    // ── Create ─────────────────────────────────────────────────────────────

    pub async fn create(&self, req: CreateJobRequest) -> Result<Job, sqlx::Error> {
        let scheduled_at = req.scheduled_at.unwrap_or_else(Utc::now);
        let priority = req.priority.unwrap_or(5);
        let max_retries = req.max_retries.unwrap_or(3);

        sqlx::query_as::<_, Job>(
            r#"
            INSERT INTO jobs (id, job_type, payload, priority, max_retries, scheduled_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING *
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(&req.job_type)
        .bind(&req.payload)
        .bind(priority)
        .bind(max_retries)
        .bind(scheduled_at)
        .fetch_one(&self.pool)
        .await
    }

    // ── Read ───────────────────────────────────────────────────────────────

    pub async fn find_by_id(&self, id: Uuid) -> Result<Option<Job>, sqlx::Error> {
        sqlx::query_as::<_, Job>("SELECT * FROM jobs WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
    }

    pub async fn find_all(&self, pagination: &PaginationParams) -> Result<PagedData<Job>, sqlx::Error> {
        let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM jobs")
            .fetch_one(&self.pool)
            .await?;

        let jobs = sqlx::query_as::<_, Job>(
            "SELECT * FROM jobs ORDER BY created_at DESC LIMIT $1 OFFSET $2",
        )
        .bind(pagination.limit())
        .bind(pagination.offset())
        .fetch_all(&self.pool)
        .await?;

        Ok(PagedData {
            data: jobs,
            total: total.0,
        })
    }

    pub async fn find_by_status(
        &self,
        status: JobStatus,
        pagination: &PaginationParams,
    ) -> Result<PagedData<Job>, sqlx::Error> {
        let total: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM jobs WHERE status = $1")
                .bind(status)
                .fetch_one(&self.pool)
                .await?;

        let jobs = sqlx::query_as::<_, Job>(
            "SELECT * FROM jobs WHERE status = $1 ORDER BY created_at DESC LIMIT $2 OFFSET $3",
        )
        .bind(status)
        .bind(pagination.limit())
        .bind(pagination.offset())
        .fetch_all(&self.pool)
        .await?;

        Ok(PagedData {
            data: jobs,
            total: total.0,
        })
    }

    // ── Cancel ─────────────────────────────────────────────────────────────
    //
    // Only pending jobs can be cancelled. If the job is already running,
    // it's too late — the worker owns it. We return the updated row so the
    // caller can verify the transition happened.

    pub async fn cancel(&self, id: Uuid) -> Result<Option<Job>, sqlx::Error> {
        sqlx::query_as::<_, Job>(
            r#"
            UPDATE jobs
            SET status = 'cancelled', updated_at = NOW()
            WHERE id = $1 AND status = 'pending'
            RETURNING *
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
    }

    // ── Stats ──────────────────────────────────────────────────────────────
    pub async fn stats(&self) -> Result<JobStats, sqlx::Error> {
        sqlx::query_as::<_, JobStats>(
            r#"
            SELECT
                COALESCE(COUNT(*) FILTER (WHERE status = 'pending'), 0) as pending,
                COALESCE(COUNT(*) FILTER (WHERE status = 'running'), 0) as running,
                COALESCE(COUNT(*) FILTER (WHERE status = 'completed'), 0) as completed,
                COALESCE(COUNT(*) FILTER (WHERE status = 'dead'), 0) as dead,
                COALESCE(COUNT(*) FILTER (WHERE status = 'cancelled'), 0) as cancelled
            FROM jobs
            "#,
        )
        .fetch_one(&self.pool)
        .await
    }

    // ── Worker methods ─────────────────────────────────────────────────────
    //
    // These will be called by the background worker, not the API.
    // We put them here because they're still database operations.
    //
    // FOR UPDATE SKIP LOCKED is the magic:
    // - FOR UPDATE: locks the selected row so no other transaction can modify it
    // - SKIP LOCKED: if the row is already locked by another worker, skip it
    //   instead of waiting. This means multiple workers never block each other.
    pub async fn acquire_next(
        &self,
        worker_id: &str,
    ) -> Result<Option<Job>, sqlx::Error> {
        // We use a transaction so the SELECT FOR UPDATE and the UPDATE
        // are atomic. If anything fails, both are rolled back.
        let mut tx = self.pool.begin().await?;

        let maybe_job = sqlx::query_as::<_, Job>(
            r#"
            SELECT * FROM jobs
            WHERE status = 'pending'
              AND scheduled_at <= NOW()
            ORDER BY priority ASC, scheduled_at ASC
            LIMIT 1
            FOR UPDATE SKIP LOCKED
            "#,
        )
        .fetch_optional(&mut *tx)
        .await?;

        let job = match maybe_job {
            Some(job) => job,
            None => {
                tx.commit().await?;
                return Ok(None);
            }
        };

        let updated = sqlx::query_as::<_, Job>(
            r#"
            UPDATE jobs
            SET status = 'running',
                locked_by = $1,
                started_at = NOW(),
                attempt = attempt + 1,
                updated_at = NOW()
            WHERE id = $2
            RETURNING *
            "#,
        )
        .bind(worker_id)
        .bind(job.id)
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(Some(updated))
    }

    /// Mark a job as completed successfully
    pub async fn complete(&self, id: Uuid) -> Result<Option<Job>, sqlx::Error> {
        sqlx::query_as::<_, Job>(
            r#"
            UPDATE jobs
            SET status = 'completed',
                completed_at = NOW(),
                updated_at = NOW()
            WHERE id = $1 AND status = 'running'
            RETURNING *
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
    }

    /// Mark a job as failed. If retries remain, set it back to pending
    /// with a backoff delay. If retries are exhausted, mark it dead.
    ///
    /// Backoff formula: base_delay * 2^(attempt - 1)
    /// Attempt 1 → 2s, attempt 2 → 4s, attempt 3 → 8s, etc.
    pub async fn fail(
        &self,
        id: Uuid,
        error_message: &str,
    ) -> Result<Option<Job>, sqlx::Error> {
        // First, get the current job state so we can check retry count
        let job = match self.find_by_id(id).await? {
            Some(j) => j,
            None => return Ok(None),
        };

        if job.attempt < job.max_retries {
            // Has retries left → back to pending with exponential backoff
            let backoff_seconds = 2_i64.pow(job.attempt as u32);

            sqlx::query_as::<_, Job>(
                r#"
                UPDATE jobs
                SET status = 'pending',
                    last_error = $1,
                    locked_by = NULL,
                    started_at = NULL,
                    scheduled_at = NOW() + ($2 || ' seconds')::INTERVAL,
                    updated_at = NOW()
                WHERE id = $3 AND status = 'running'
                RETURNING *
                "#,
            )
            .bind(error_message)
            .bind(backoff_seconds.to_string())
            .bind(id)
            .fetch_optional(&self.pool)
            .await
        } else {
            // No retries left → dead
            sqlx::query_as::<_, Job>(
                r#"
                UPDATE jobs
                SET status = 'dead',
                    last_error = $1,
                    completed_at = NOW(),
                    updated_at = NOW()
                WHERE id = $2 AND status = 'running'
                RETURNING *
                "#,
            )
            .bind(error_message)
            .bind(id)
            .fetch_optional(&self.pool)
            .await
        }
    }

    // ── Reaper methods ─────────────────────────────────────────────────────

    /// Find jobs stuck in 'running' longer than `threshold_secs` and
    /// reset them to 'pending'. Returns how many jobs were reset.
    ///
    /// These are jobs whose worker likely crashed. By resetting them,
    /// a healthy worker can pick them up on the next poll cycle.
    /// We also clear `locked_by` and `started_at` since the original
    /// worker is gone.
    pub async fn reset_stale_jobs(&self, threshold_secs: i64) -> Result<i64, sqlx::Error> {
        let result = sqlx::query(
            r#"
            UPDATE jobs
            SET status = 'pending',
                locked_by = NULL,
                started_at = NULL,
                updated_at = NOW(),
                last_error = COALESCE(last_error, '') || ' [reset by reaper: worker presumed dead]'
            WHERE status = 'running'
              AND started_at < NOW() - ($1 || ' seconds')::INTERVAL
            "#,
        )
        .bind(threshold_secs.to_string())
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() as i64)
    }
}
