use axum::{
    Json, Router,
    extract::{Path, Query, State},
    response::IntoResponse,
    routing::{get, post},
};
use uuid::Uuid;

use crate::error::AppError;
use crate::models::job::{CreateJobRequest, JobDetailResponse, JobMetrics, JobResponse, JobStats, JobStatus};
use crate::models::pagination::{PaginatedResponse, PaginationParams};
use crate::models::responses::DataResponse;
use crate::state::AppState;
use crate::validation::validate;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", post(create_job).get(list_jobs))
        .route("/stats", get(get_stats))
        .route("/metrics", get(get_metrics))
        .route("/{id}", get(get_job))
        .route("/{id}/cancel", post(cancel_job))
}

// ── Submit a job ───────────────────────────────────────────────────────────

#[utoipa::path(
    post,
    path = "/api/jobs",
    request_body = CreateJobRequest,
    responses(
        (status = 201, description = "Job created", body = DataResponse<JobResponse>),
        (status = 400, description = "Invalid request"),
    ),
    security(("bearer_auth" = [])),
    tag = "Jobs"
)]
async fn create_job(
    State(state): State<AppState>,
    Json(payload): Json<CreateJobRequest>,
) -> Result<impl IntoResponse, AppError> {
    validate(&payload)?;

    let job = state.jobs.create(payload).await?;

    Ok((
        axum::http::StatusCode::CREATED,
        Json(DataResponse::new(JobResponse::from(job))),
    ))
}

// ── Get a single job ───────────────────────────────────────────────────────
//
// Returns the full detail response since this is behind auth
// and is primarily for the dashboard.

#[utoipa::path(
    get,
    path = "/api/jobs/{id}",
    responses(
        (status = 200, description = "Job details", body = DataResponse<JobDetailResponse>),
        (status = 404, description = "Job not found"),
    ),
    params(
        ("id" = Uuid, Path, description = "Job ID")
    ),
    security(("bearer_auth" = [])),
    tag = "Jobs"
)]
async fn get_job(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let job = state
        .jobs
        .find_by_id(id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Job {} not found", id)))?;

    Ok(Json(DataResponse::new(JobDetailResponse::from(job))))
}

// ── List jobs ──────────────────────────────────────────────────────────────
//
// Optional `status` query param to filter by status.
// e.g. GET /api/jobs?status=pending&limit=10

#[derive(Debug, serde::Deserialize, utoipa::IntoParams)]
struct ListJobsParams {
    /// Filter by job status
    status: Option<JobStatus>,
}

#[utoipa::path(
    get,
    path = "/api/jobs",
    params(PaginationParams, ListJobsParams),
    responses(
        (status = 200, description = "Paginated list of jobs"),
    ),
    security(("bearer_auth" = [])),
    tag = "Jobs"
)]
async fn list_jobs(
    State(state): State<AppState>,
    Query(pagination): Query<PaginationParams>,
    Query(filter): Query<ListJobsParams>,
) -> Result<impl IntoResponse, AppError> {
    let result = match filter.status {
        Some(status) => state.jobs.find_by_status(status, &pagination).await?,
        None => state.jobs.find_all(&pagination).await?,
    };

    let response = PaginatedResponse::new(
        result.data.into_iter().map(JobResponse::from).collect(),
        result.total,
        &pagination,
    );

    Ok(Json(response))
}

// ── Cancel a job ───────────────────────────────────────────────────────────

#[utoipa::path(
    post,
    path = "/api/jobs/{id}/cancel",
    responses(
        (status = 200, description = "Job cancelled", body = DataResponse<JobResponse>),
        (status = 404, description = "Job not found"),
        (status = 409, description = "Job cannot be cancelled (not in pending state)"),
    ),
    params(
        ("id" = Uuid, Path, description = "Job ID")
    ),
    security(("bearer_auth" = [])),
    tag = "Jobs"
)]
async fn cancel_job(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    // First check the job exists at all
    let job = state
        .jobs
        .find_by_id(id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Job {} not found", id)))?;

    // If it's not pending, we can't cancel it
    if job.status != JobStatus::Pending {
        return Err(AppError::Conflict(format!(
            "Job {} cannot be cancelled — current status is '{}'",
            id, job.status
        )));
    }

    let cancelled = state
        .jobs
        .cancel(id)
        .await?
        .ok_or_else(|| {
            // Race condition: job was claimed between our check and the cancel.
            AppError::Conflict(format!(
                "Job {} was claimed by a worker before it could be cancelled",
                id
            ))
        })?;

    Ok(Json(DataResponse::new(JobResponse::from(cancelled))))
}

// ── Stats ──────────────────────────────────────────────────────────────────

#[utoipa::path(
    get,
    path = "/api/jobs/stats",
    responses(
        (status = 200, description = "Job queue statistics", body = DataResponse<JobStats>),
    ),
    security(("bearer_auth" = [])),
    tag = "Jobs"
)]
async fn get_stats(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    let stats = state.jobs.stats().await?;
    Ok(Json(DataResponse::new(stats)))
}

// ── Metrics ─────────────────────────────────────────────────────────────────
//
// Richer than /stats — includes processing times, throughput, retry rates.
// We fire all queries concurrently with tokio::try_join! since they're
// independent. This keeps the endpoint fast even as the data grows.

#[utoipa::path(
    get,
    path = "/api/jobs/metrics",
    responses(
        (status = 200, description = "Detailed queue metrics", body = DataResponse<JobMetrics>),
    ),
    security(("bearer_auth" = [])),
    tag = "Jobs"
)]
async fn get_metrics(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    let (counts, avg_duration, throughput, retry_rate, by_type) = tokio::try_join!(
        state.jobs.stats(),
        state.jobs.avg_duration(),
        state.jobs.throughput(),
        state.jobs.retry_rate(),
        state.jobs.stats_by_type(),
    )?;

    let metrics = JobMetrics {
        counts,
        avg_duration_secs: avg_duration,
        throughput,
        retry_rate,
        by_type,
    };

    Ok(Json(DataResponse::new(metrics)))
}
