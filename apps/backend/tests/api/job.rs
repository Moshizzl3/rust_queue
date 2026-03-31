use axum::http::StatusCode;
use serde_json::json;
use tower::ServiceExt;

use crate::common::helpers::{TestApp, auth_request, create_authenticated_user, response_json};

// ==================== Create Job ====================

#[tokio::test]
async fn test_create_job() {
    let app = TestApp::spawn().await;
    let (_, token) = create_authenticated_user(&app.state).await;

    let response = auth_request(
        &app.state,
        &token,
        "POST",
        "/api/jobs",
        Some(json!({
            "job_type": "fast_task",
            "payload": {"key": "value"}
        })),
    )
    .await;

    assert_eq!(response.status(), StatusCode::CREATED);

    let json = response_json(response).await;
    let job = json.get("data").unwrap();

    assert_eq!(job.get("job_type").unwrap(), "fast_task");
    assert_eq!(job.get("status").unwrap(), "pending");
    assert_eq!(job.get("priority").unwrap(), 5);
    assert_eq!(job.get("attempt").unwrap(), 0);
    assert_eq!(job.get("max_retries").unwrap(), 3);
    assert_eq!(job.get("payload").unwrap().get("key").unwrap(), "value");
}

#[tokio::test]
async fn test_create_job_with_priority() {
    let app = TestApp::spawn().await;
    let (_, token) = create_authenticated_user(&app.state).await;

    let response = auth_request(
        &app.state,
        &token,
        "POST",
        "/api/jobs",
        Some(json!({
            "job_type": "critical_report",
            "priority": 1
        })),
    )
    .await;

    assert_eq!(response.status(), StatusCode::CREATED);

    let json = response_json(response).await;
    let job = json.get("data").unwrap();

    assert_eq!(job.get("priority").unwrap(), 1);
}

#[tokio::test]
async fn test_create_job_with_custom_retries() {
    let app = TestApp::spawn().await;
    let (_, token) = create_authenticated_user(&app.state).await;

    let response = auth_request(
        &app.state,
        &token,
        "POST",
        "/api/jobs",
        Some(json!({
            "job_type": "flaky_task",
            "max_retries": 10
        })),
    )
    .await;

    assert_eq!(response.status(), StatusCode::CREATED);

    let json = response_json(response).await;
    let job = json.get("data").unwrap();

    assert_eq!(job.get("max_retries").unwrap(), 10);
}

#[tokio::test]
async fn test_create_job_defaults() {
    let app = TestApp::spawn().await;
    let (_, token) = create_authenticated_user(&app.state).await;

    let response = auth_request(
        &app.state,
        &token,
        "POST",
        "/api/jobs",
        Some(json!({
            "job_type": "fast_task"
        })),
    )
    .await;

    assert_eq!(response.status(), StatusCode::CREATED);

    let json = response_json(response).await;
    let job = json.get("data").unwrap();

    // Check defaults are applied
    assert_eq!(job.get("priority").unwrap(), 5);
    assert_eq!(job.get("max_retries").unwrap(), 3);
    assert_eq!(job.get("payload").unwrap(), &json!({}));
}

#[tokio::test]
async fn test_create_job_invalid_priority() {
    let app = TestApp::spawn().await;
    let (_, token) = create_authenticated_user(&app.state).await;

    let response = auth_request(
        &app.state,
        &token,
        "POST",
        "/api/jobs",
        Some(json!({
            "job_type": "fast_task",
            "priority": 99
        })),
    )
    .await;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_create_job_empty_job_type() {
    let app = TestApp::spawn().await;
    let (_, token) = create_authenticated_user(&app.state).await;

    let response = auth_request(
        &app.state,
        &token,
        "POST",
        "/api/jobs",
        Some(json!({
            "job_type": ""
        })),
    )
    .await;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_create_job_unauthorized() {
    let app = TestApp::spawn().await;

    let router = rust_queue::build_router(app.state.clone());
    let response = router
        .oneshot(
            axum::http::Request::builder()
                .method("POST")
                .uri("/api/jobs")
                .header("Content-Type", "application/json")
                .body(axum::body::Body::from(
                    serde_json::to_string(&json!({"job_type": "fast_task"})).unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

// ==================== Get Job ====================

#[tokio::test]
async fn test_get_job() {
    let app = TestApp::spawn().await;
    let (_, token) = create_authenticated_user(&app.state).await;

    // Create a job first
    let create_response = auth_request(
        &app.state,
        &token,
        "POST",
        "/api/jobs",
        Some(json!({"job_type": "slow_task"})),
    )
    .await;

    let create_json = response_json(create_response).await;
    let job_id = create_json["data"]["id"].as_str().unwrap();

    // Fetch it
    let response = auth_request(
        &app.state,
        &token,
        "GET",
        &format!("/api/jobs/{}", job_id),
        None,
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);

    let json = response_json(response).await;
    let job = json.get("data").unwrap();

    assert_eq!(job.get("id").unwrap(), job_id);
    assert_eq!(job.get("job_type").unwrap(), "slow_task");
    // Detail response should include internal fields
    assert!(job.get("locked_by").is_some());
    assert!(job.get("updated_at").is_some());
}

#[tokio::test]
async fn test_get_job_not_found() {
    let app = TestApp::spawn().await;
    let (_, token) = create_authenticated_user(&app.state).await;

    let fake_id = uuid::Uuid::new_v4();
    let response = auth_request(
        &app.state,
        &token,
        "GET",
        &format!("/api/jobs/{}", fake_id),
        None,
    )
    .await;

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

// ==================== List Jobs ====================

#[tokio::test]
async fn test_list_jobs_empty() {
    let app = TestApp::spawn().await;
    let (_, token) = create_authenticated_user(&app.state).await;

    let response = auth_request(&app.state, &token, "GET", "/api/jobs", None).await;

    assert_eq!(response.status(), StatusCode::OK);

    let json = response_json(response).await;

    assert_eq!(json["data"].as_array().unwrap().len(), 0);
    assert_eq!(json["pagination"]["total"].as_i64().unwrap(), 0);
}

#[tokio::test]
async fn test_list_jobs_with_data() {
    let app = TestApp::spawn().await;
    let (_, token) = create_authenticated_user(&app.state).await;

    // Create 3 jobs
    for job_type in &["fast_task", "slow_task", "flaky_task"] {
        auth_request(
            &app.state,
            &token,
            "POST",
            "/api/jobs",
            Some(json!({"job_type": job_type})),
        )
        .await;
    }

    let response = auth_request(&app.state, &token, "GET", "/api/jobs", None).await;

    assert_eq!(response.status(), StatusCode::OK);

    let json = response_json(response).await;

    assert_eq!(json["data"].as_array().unwrap().len(), 3);
    assert_eq!(json["pagination"]["total"].as_i64().unwrap(), 3);
}

#[tokio::test]
async fn test_list_jobs_pagination() {
    let app = TestApp::spawn().await;
    let (_, token) = create_authenticated_user(&app.state).await;

    // Create 5 jobs
    for _ in 0..5 {
        auth_request(
            &app.state,
            &token,
            "POST",
            "/api/jobs",
            Some(json!({"job_type": "fast_task"})),
        )
        .await;
    }

    // Request only 2
    let response = auth_request(&app.state, &token, "GET", "/api/jobs?limit=2", None).await;

    let json = response_json(response).await;

    assert_eq!(json["data"].as_array().unwrap().len(), 2);
    assert_eq!(json["pagination"]["total"].as_i64().unwrap(), 5);
    assert_eq!(json["pagination"]["has_more"].as_bool().unwrap(), true);
}

#[tokio::test]
async fn test_list_jobs_filter_by_status() {
    let app = TestApp::spawn().await;
    let (_, token) = create_authenticated_user(&app.state).await;

    // Create 2 pending jobs
    auth_request(
        &app.state,
        &token,
        "POST",
        "/api/jobs",
        Some(json!({"job_type": "fast_task"})),
    )
    .await;
    auth_request(
        &app.state,
        &token,
        "POST",
        "/api/jobs",
        Some(json!({"job_type": "slow_task"})),
    )
    .await;

    // Cancel one
    let create_resp = auth_request(
        &app.state,
        &token,
        "POST",
        "/api/jobs",
        Some(json!({"job_type": "flaky_task"})),
    )
    .await;
    let create_json = response_json(create_resp).await;
    let cancel_id = create_json["data"]["id"].as_str().unwrap();
    auth_request(
        &app.state,
        &token,
        "POST",
        &format!("/api/jobs/{}/cancel", cancel_id),
        None,
    )
    .await;

    // Filter by pending — should be 2
    let response = auth_request(&app.state, &token, "GET", "/api/jobs?status=pending", None).await;

    let json = response_json(response).await;
    assert_eq!(json["pagination"]["total"].as_i64().unwrap(), 2);

    // Filter by cancelled — should be 1
    let response = auth_request(
        &app.state,
        &token,
        "GET",
        "/api/jobs?status=cancelled",
        None,
    )
    .await;

    let json = response_json(response).await;
    assert_eq!(json["pagination"]["total"].as_i64().unwrap(), 1);
}

// ==================== Cancel Job ====================

#[tokio::test]
async fn test_cancel_pending_job() {
    let app = TestApp::spawn().await;
    let (_, token) = create_authenticated_user(&app.state).await;

    let create_resp = auth_request(
        &app.state,
        &token,
        "POST",
        "/api/jobs",
        Some(json!({"job_type": "slow_task"})),
    )
    .await;

    let create_json = response_json(create_resp).await;
    let job_id = create_json["data"]["id"].as_str().unwrap();

    let response = auth_request(
        &app.state,
        &token,
        "POST",
        &format!("/api/jobs/{}/cancel", job_id),
        None,
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);

    let json = response_json(response).await;
    assert_eq!(json["data"]["status"].as_str().unwrap(), "cancelled");
}

#[tokio::test]
async fn test_cancel_nonexistent_job() {
    let app = TestApp::spawn().await;
    let (_, token) = create_authenticated_user(&app.state).await;

    let fake_id = uuid::Uuid::new_v4();
    let response = auth_request(
        &app.state,
        &token,
        "POST",
        &format!("/api/jobs/{}/cancel", fake_id),
        None,
    )
    .await;

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_cancel_already_cancelled_job() {
    let app = TestApp::spawn().await;
    let (_, token) = create_authenticated_user(&app.state).await;

    // Create and cancel
    let create_resp = auth_request(
        &app.state,
        &token,
        "POST",
        "/api/jobs",
        Some(json!({"job_type": "fast_task"})),
    )
    .await;
    let create_json = response_json(create_resp).await;
    let job_id = create_json["data"]["id"].as_str().unwrap();

    auth_request(
        &app.state,
        &token,
        "POST",
        &format!("/api/jobs/{}/cancel", job_id),
        None,
    )
    .await;

    // Try to cancel again
    let response = auth_request(
        &app.state,
        &token,
        "POST",
        &format!("/api/jobs/{}/cancel", job_id),
        None,
    )
    .await;

    assert_eq!(response.status(), StatusCode::CONFLICT);
}

// ==================== Stats ====================

#[tokio::test]
async fn test_stats_empty() {
    let app = TestApp::spawn().await;
    let (_, token) = create_authenticated_user(&app.state).await;

    let response = auth_request(&app.state, &token, "GET", "/api/jobs/stats", None).await;

    assert_eq!(response.status(), StatusCode::OK);

    let json = response_json(response).await;
    let stats = json.get("data").unwrap();

    assert_eq!(stats["pending"].as_i64().unwrap(), 0);
    assert_eq!(stats["running"].as_i64().unwrap(), 0);
    assert_eq!(stats["completed"].as_i64().unwrap(), 0);
    assert_eq!(stats["dead"].as_i64().unwrap(), 0);
    assert_eq!(stats["cancelled"].as_i64().unwrap(), 0);
}

#[tokio::test]
async fn test_stats_counts_correctly() {
    let app = TestApp::spawn().await;
    let (_, token) = create_authenticated_user(&app.state).await;

    // Create 3 pending jobs
    for _ in 0..3 {
        auth_request(
            &app.state,
            &token,
            "POST",
            "/api/jobs",
            Some(json!({"job_type": "fast_task"})),
        )
        .await;
    }

    // Cancel 1
    let create_resp = auth_request(
        &app.state,
        &token,
        "POST",
        "/api/jobs",
        Some(json!({"job_type": "fast_task"})),
    )
    .await;
    let create_json = response_json(create_resp).await;
    let job_id = create_json["data"]["id"].as_str().unwrap();
    auth_request(
        &app.state,
        &token,
        "POST",
        &format!("/api/jobs/{}/cancel", job_id),
        None,
    )
    .await;

    let response = auth_request(&app.state, &token, "GET", "/api/jobs/stats", None).await;

    let json = response_json(response).await;
    let stats = json.get("data").unwrap();

    assert_eq!(stats["pending"].as_i64().unwrap(), 3);
    assert_eq!(stats["cancelled"].as_i64().unwrap(), 1);
}

// ==================== Repository-level tests ====================
// These test the worker-facing methods directly (acquire, complete, fail)
// without going through the HTTP layer.

#[tokio::test]
async fn test_acquire_next_returns_highest_priority() {
    let app = TestApp::spawn().await;

    // Create jobs with different priorities
    let low = json!({"job_type": "fast_task", "priority": 10});
    let high = json!({"job_type": "critical_report", "priority": 1});

    app.state
        .jobs
        .create(serde_json::from_value(low).unwrap())
        .await
        .unwrap();
    app.state
        .jobs
        .create(serde_json::from_value(high).unwrap())
        .await
        .unwrap();

    // Worker should pick the high-priority job first
    let job = app
        .state
        .jobs
        .acquire_next("test-worker")
        .await
        .unwrap()
        .unwrap();

    assert_eq!(job.job_type, "critical_report");
    assert_eq!(job.priority, 1);
    assert_eq!(job.status, rust_queue::models::job::JobStatus::Running);
    assert_eq!(job.locked_by.as_deref(), Some("test-worker"));
    assert_eq!(job.attempt, 1);
}

#[tokio::test]
async fn test_acquire_next_returns_none_when_empty() {
    let app = TestApp::spawn().await;

    let job = app.state.jobs.acquire_next("test-worker").await.unwrap();
    assert!(job.is_none());
}

#[tokio::test]
async fn test_acquire_skips_running_jobs() {
    let app = TestApp::spawn().await;

    let req = serde_json::from_value(json!({"job_type": "fast_task"})).unwrap();
    app.state.jobs.create(req).await.unwrap();

    // First worker claims it
    let job = app.state.jobs.acquire_next("worker-1").await.unwrap();
    assert!(job.is_some());

    // Second worker should get nothing
    let job = app.state.jobs.acquire_next("worker-2").await.unwrap();
    assert!(job.is_none());
}

#[tokio::test]
async fn test_complete_job() {
    let app = TestApp::spawn().await;

    let req = serde_json::from_value(json!({"job_type": "fast_task"})).unwrap();
    app.state.jobs.create(req).await.unwrap();

    let job = app
        .state
        .jobs
        .acquire_next("test-worker")
        .await
        .unwrap()
        .unwrap();
    let completed = app.state.jobs.complete(job.id).await.unwrap().unwrap();

    assert_eq!(
        completed.status,
        rust_queue::models::job::JobStatus::Completed
    );
    assert!(completed.completed_at.is_some());
}

#[tokio::test]
async fn test_fail_job_with_retries_remaining() {
    let app = TestApp::spawn().await;

    let req = serde_json::from_value(json!({
        "job_type": "flaky_task",
        "max_retries": 3
    }))
    .unwrap();
    app.state.jobs.create(req).await.unwrap();

    // Acquire and fail
    let job = app
        .state
        .jobs
        .acquire_next("test-worker")
        .await
        .unwrap()
        .unwrap();
    let failed = app
        .state
        .jobs
        .fail(job.id, "something went wrong")
        .await
        .unwrap()
        .unwrap();

    // Should be back to pending with the error recorded
    assert_eq!(failed.status, rust_queue::models::job::JobStatus::Pending);
    assert_eq!(failed.last_error.as_deref(), Some("something went wrong"));
    assert!(failed.locked_by.is_none()); // lock released
    assert!(failed.started_at.is_none()); // reset for next attempt
    // scheduled_at should be in the future (backoff)
    assert!(failed.scheduled_at > chrono::Utc::now() - chrono::Duration::seconds(1));
}

#[tokio::test]
async fn test_fail_job_exhausts_retries_becomes_dead() {
    let app = TestApp::spawn().await;

    // Create with max_retries = 1
    let req = serde_json::from_value(json!({
        "job_type": "flaky_task",
        "max_retries": 1
    }))
    .unwrap();
    app.state.jobs.create(req).await.unwrap();

    // Attempt 1: acquire and fail
    let job = app
        .state
        .jobs
        .acquire_next("test-worker")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(job.attempt, 1);
    let failed = app
        .state
        .jobs
        .fail(job.id, "first failure")
        .await
        .unwrap()
        .unwrap();

    // attempt=1, max_retries=1 → exhausted → dead
    assert_eq!(failed.status, rust_queue::models::job::JobStatus::Dead);
    assert_eq!(failed.last_error.as_deref(), Some("first failure"));
    assert!(failed.completed_at.is_some());
}

#[tokio::test]
async fn test_reset_stale_jobs() {
    let app = TestApp::spawn().await;

    let req = serde_json::from_value(json!({"job_type": "slow_task"})).unwrap();
    app.state.jobs.create(req).await.unwrap();

    // Acquire it (now it's running)
    let job = app
        .state
        .jobs
        .acquire_next("dead-worker")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(job.status, rust_queue::models::job::JobStatus::Running);

    // Manually backdate started_at to simulate a stale job.
    // Set it 10 minutes in the past.
    sqlx::query("UPDATE jobs SET started_at = NOW() - INTERVAL '10 minutes' WHERE id = $1")
        .bind(job.id)
        .execute(&app.state.pool)
        .await
        .unwrap();

    // Reaper with 5-minute threshold should catch it
    let count = app.state.jobs.reset_stale_jobs(300).await.unwrap();
    assert_eq!(count, 1);

    // Job should be back to pending
    let recovered = app.state.jobs.find_by_id(job.id).await.unwrap().unwrap();
    assert_eq!(
        recovered.status,
        rust_queue::models::job::JobStatus::Pending
    );
    assert!(recovered.locked_by.is_none());
    assert!(recovered.last_error.unwrap().contains("reset by reaper"));
}

#[tokio::test]
async fn test_reset_stale_jobs_ignores_recent() {
    let app = TestApp::spawn().await;

    let req = serde_json::from_value(json!({"job_type": "slow_task"})).unwrap();
    app.state.jobs.create(req).await.unwrap();

    // Acquire it (just started — not stale)
    app.state
        .jobs
        .acquire_next("active-worker")
        .await
        .unwrap()
        .unwrap();

    // Reaper should NOT reset it
    let count = app.state.jobs.reset_stale_jobs(300).await.unwrap();
    assert_eq!(count, 0);
}
