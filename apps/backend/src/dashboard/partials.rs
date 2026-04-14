use axum::{
    Form,
    extract::State,
    http::header,
    response::{Html, IntoResponse, Response},
};
use serde::Deserialize;

use crate::models::job::CreateJobRequest;
use crate::state::AppState;
use super::pages::get_current_user;

/// Guard for partials — if not authenticated, tell htmx to redirect.
async fn require_auth(state: &AppState, headers: &axum::http::HeaderMap) -> Option<Response> {
    if get_current_user(state, headers).await.is_none() {
        Some(
            (
                [(header::HeaderName::from_static("hx-redirect"), "/dashboard/login".to_string())],
                "",
            )
                .into_response(),
        )
    } else {
        None
    }
}

/// Stats cards fragment — polled every 3s
pub async fn stats_fragment(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> Response {
    if let Some(redirect) = require_auth(&state, &headers).await {
        return redirect;
    }

    let stats = match state.jobs.stats().await {
        Ok(s) => s,
        Err(_) => return Html("<p>Error loading stats</p>".to_string()).into_response(),
    };

    Html(format!(
        r#"<div class="stats-grid">
            <div class="stat-card pending">
                <div class="label">Pending</div>
                <div class="value">{}</div>
            </div>
            <div class="stat-card running">
                <div class="label">Running</div>
                <div class="value">{}</div>
            </div>
            <div class="stat-card completed">
                <div class="label">Completed</div>
                <div class="value">{}</div>
            </div>
            <div class="stat-card dead">
                <div class="label">Dead</div>
                <div class="value">{}</div>
            </div>
            <div class="stat-card cancelled">
                <div class="label">Cancelled</div>
                <div class="value">{}</div>
            </div>
        </div>"#,
        stats.pending, stats.running, stats.completed, stats.dead, stats.cancelled
    ))
    .into_response()
}

/// Metrics fragment — polled every 5s
pub async fn metrics_fragment(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> Response {
    if let Some(redirect) = require_auth(&state, &headers).await {
        return redirect;
    }

    let (avg_duration, throughput, retry_rate) = match tokio::try_join!(
        state.jobs.avg_duration(),
        state.jobs.throughput(),
        state.jobs.retry_rate(),
    ) {
        Ok(data) => data,
        Err(_) => return Html("<p>Error loading metrics</p>".to_string()).into_response(),
    };

    let avg_str = avg_duration
        .map(|d| format!("{:.1}s", d))
        .unwrap_or_else(|| "—".to_string());

    Html(format!(
        r#"<div class="metrics-grid">
            <div class="metric-card">
                <div class="label">Avg Processing Time</div>
                <div class="value">{avg_str}</div>
            </div>
            <div class="metric-card">
                <div class="label">Throughput</div>
                <div class="value">{} / min</div>
                <div class="detail">{} last 5m · {} last 1h</div>
            </div>
            <div class="metric-card">
                <div class="label">Retry Rate</div>
                <div class="value">{:.1}%</div>
                <div class="detail">of completed jobs needed retries</div>
            </div>
        </div>"#,
        throughput.last_1m,
        throughput.last_5m,
        throughput.last_1h,
        retry_rate
    ))
    .into_response()
}

/// Jobs table fragment — polled every 3s
pub async fn jobs_fragment(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> Response {
    if let Some(redirect) = require_auth(&state, &headers).await {
        return redirect;
    }

    let pagination = crate::models::pagination::PaginationParams {
        limit: Some(25),
        offset: Some(0),
        sort_by: Some("created_at".to_string()),
        sort_order: Some(crate::models::pagination::SortOrder::Desc),
    };

    let jobs = match state.jobs.find_all(&pagination).await {
        Ok(result) => result.data,
        Err(_) => return Html("<p>Error loading jobs</p>".to_string()).into_response(),
    };

    if jobs.is_empty() {
        return Html(r#"<p style="color: #555; font-size: 13px; padding: 16px 0;">No jobs yet. Submit one above.</p>"#.to_string()).into_response();
    }

    let mut rows = String::new();
    for job in &jobs {
        let status_class = format!("{}", job.status).to_lowercase();
        let ago = format_ago(job.created_at);
        let error_cell = job
            .last_error
            .as_deref()
            .map(|e| {
                let truncated = if e.len() > 60 { &e[..60] } else { e };
                format!(r#"<span style="color: #ef4444; font-size: 11px;">{}</span>"#, truncated)
            })
            .unwrap_or_default();

        rows.push_str(&format!(
            r#"<tr>
                <td class="mono">{}</td>
                <td>{}</td>
                <td><span class="status {status_class}">{status_class}</span></td>
                <td>{}</td>
                <td>{}/{}</td>
                <td>{error_cell}</td>
                <td style="color: #555;">{ago}</td>
            </tr>"#,
            &job.id.to_string()[..8],
            job.job_type,
            job.priority,
            job.attempt,
            job.max_retries,
        ));
    }

    Html(format!(
        r#"<table>
            <thead>
                <tr>
                    <th>ID</th>
                    <th>Type</th>
                    <th>Status</th>
                    <th>Priority</th>
                    <th>Attempts</th>
                    <th>Error</th>
                    <th>Created</th>
                </tr>
            </thead>
            <tbody>{rows}</tbody>
        </table>"#
    ))
    .into_response()
}

/// Submit form fragment
pub async fn submit_form_fragment(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> Response {
    if let Some(redirect) = require_auth(&state, &headers).await {
        return redirect;
    }

    Html(r##"<form class="submit-form"
          hx-post="/dashboard/partials/submit"
          hx-target="closest .section"
          hx-swap="innerHTML">
        <div class="form-group">
            <label>Job Type</label>
            <select name="job_type">
                <option value="fast_task">fast_task</option>
                <option value="slow_task">slow_task</option>
                <option value="flaky_task">flaky_task</option>
                <option value="critical_report">critical_report</option>
            </select>
        </div>
        <div class="form-group">
            <label>Priority (0-10)</label>
            <input type="number" name="priority" value="5" min="0" max="10" style="width: 80px;">
        </div>
        <div class="form-group">
            <label>Max Retries</label>
            <input type="number" name="max_retries" value="3" min="0" max="20" style="width: 80px;">
        </div>
        <button type="submit">Submit Job</button>
    </form>"##.to_string())
    .into_response()
}

/// Handle job submission from the dashboard form
#[derive(Deserialize)]
pub struct SubmitJobForm {
    job_type: String,
    priority: Option<i16>,
    max_retries: Option<i32>,
}

pub async fn submit_job(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Form(form): Form<SubmitJobForm>,
) -> Response {
    if let Some(redirect) = require_auth(&state, &headers).await {
        return redirect;
    }

    let req = CreateJobRequest {
        job_type: form.job_type.clone(),
        payload: serde_json::json!({}),
        priority: form.priority,
        max_retries: form.max_retries,
        scheduled_at: None,
    };

    match state.jobs.create(req).await {
        Ok(job) => {
            Html(format!(
                r#"<div class="flash">Submitted {} — {}</div>
                <form class="submit-form"
                      hx-post="/dashboard/partials/submit"
                      hx-target="closest .section"
                      hx-swap="innerHTML">
                    <div class="form-group">
                        <label>Job Type</label>
                        <select name="job_type">
                            <option value="fast_task" {}>fast_task</option>
                            <option value="slow_task" {}>slow_task</option>
                            <option value="flaky_task" {}>flaky_task</option>
                            <option value="critical_report" {}>critical_report</option>
                        </select>
                    </div>
                    <div class="form-group">
                        <label>Priority (0-10)</label>
                        <input type="number" name="priority" value="{}" min="0" max="10" style="width: 80px;">
                    </div>
                    <div class="form-group">
                        <label>Max Retries</label>
                        <input type="number" name="max_retries" value="{}" min="0" max="20" style="width: 80px;">
                    </div>
                    <button type="submit">Submit Job</button>
                </form>"#,
                form.job_type,
                &job.id.to_string()[..8],
                if form.job_type == "fast_task" { "selected" } else { "" },
                if form.job_type == "slow_task" { "selected" } else { "" },
                if form.job_type == "flaky_task" { "selected" } else { "" },
                if form.job_type == "critical_report" { "selected" } else { "" },
                form.priority.unwrap_or(5),
                form.max_retries.unwrap_or(3),
            ))
            .into_response()
        }
        Err(_) => {
            Html(r#"<div style="color: #ef4444; font-size: 13px;">Failed to submit job</div>"#.to_string())
                .into_response()
        }
    }
}

/// Format a timestamp as a human-readable "ago" string
fn format_ago(time: chrono::DateTime<chrono::Utc>) -> String {
    let now = chrono::Utc::now();
    let diff = now - time;

    if diff.num_seconds() < 60 {
        format!("{}s ago", diff.num_seconds())
    } else if diff.num_minutes() < 60 {
        format!("{}m ago", diff.num_minutes())
    } else if diff.num_hours() < 24 {
        format!("{}h ago", diff.num_hours())
    } else {
        format!("{}d ago", diff.num_days())
    }
}
