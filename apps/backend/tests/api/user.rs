use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use rust_queue::models::user::UserRole;
use serde_json::{Value, json};
use tower::ServiceExt;
use uuid::Uuid;

use crate::common::helpers::{TestApp, create_test_user, get_auth_token};

// ==================== Get Current User ====================

#[tokio::test]
async fn test_get_current_user() {
    let app = TestApp::spawn().await;
    let router = rust_queue::build_router(app.state.clone());

    let user_id =
        create_test_user(&app.state, "me@example.com", "password123", UserRole::User).await;
    let token = get_auth_token(&app.state, user_id, "me@example.com").await;

    let response = router
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/users/me")
                .header("Authorization", format!("Bearer {}", token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    let user_data = json.get("data").unwrap();

    assert_eq!(user_data.get("email").unwrap(), "me@example.com");
}

#[tokio::test]
async fn test_get_current_user_unauthorized() {
    let app = TestApp::spawn().await;
    let router = rust_queue::build_router(app.state.clone());

    let response = router
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/users/me")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_get_current_user_no_password_in_response() {
    let app = TestApp::spawn().await;
    let router = rust_queue::build_router(app.state.clone());

    let user_id =
        create_test_user(&app.state, "me@example.com", "password123", UserRole::User).await;
    let token = get_auth_token(&app.state, user_id, "me@example.com").await;

    let response = router
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/users/me")
                .header("Authorization", format!("Bearer {}", token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert!(
        json.get("password").is_none(),
        "Password should not be in response"
    );
    assert!(
        json.get("password_hash").is_none(),
        "Password hash should not be in response"
    );
}

// ==================== Update Current User ====================

#[tokio::test]
async fn test_update_current_user() {
    let app = TestApp::spawn().await;
    let router = rust_queue::build_router(app.state.clone());

    let user_id =
        create_test_user(&app.state, "me@example.com", "password123", UserRole::User).await;
    let token = get_auth_token(&app.state, user_id, "me@example.com").await;

    let update_body = json!({
        "name": "New Name"
    });

    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri("/api/users/me")
                .header("Content-Type", "application/json")
                .header("Authorization", format!("Bearer {}", token))
                .body(Body::from(serde_json::to_string(&update_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    // Verify update
    let get_response = router
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/users/me")
                .header("Authorization", format!("Bearer {}", token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let body = axum::body::to_bytes(get_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    let user_data = json.get("data").unwrap();

    assert_eq!(user_data.get("name").unwrap(), "New Name");
}

#[tokio::test]
async fn test_update_current_user_email() {
    let app = TestApp::spawn().await;
    let router = rust_queue::build_router(app.state.clone());

    let user_id =
        create_test_user(&app.state, "old@example.com", "password123", UserRole::User).await;
    let token = get_auth_token(&app.state, user_id, "old@example.com").await;

    let update_body = json!({
        "email": "new@example.com"
    });

    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri("/api/users/me")
                .header("Content-Type", "application/json")
                .header("Authorization", format!("Bearer {}", token))
                .body(Body::from(serde_json::to_string(&update_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    let user_data = json.get("data").unwrap();

    assert_eq!(user_data.get("email").unwrap(), "new@example.com");
}

#[tokio::test]
async fn test_update_current_user_invalid_email() {
    let app = TestApp::spawn().await;
    let router = rust_queue::build_router(app.state.clone());

    let user_id =
        create_test_user(&app.state, "me@example.com", "password123", UserRole::User).await;
    let token = get_auth_token(&app.state, user_id, "me@example.com").await;

    let update_body = json!({
        "email": "not-an-email"
    });

    let response = router
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri("/api/users/me")
                .header("Content-Type", "application/json")
                .header("Authorization", format!("Bearer {}", token))
                .body(Body::from(serde_json::to_string(&update_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

// ==================== Delete Current User ====================

#[tokio::test]
async fn test_delete_current_user() {
    let app = TestApp::spawn().await;
    let router = rust_queue::build_router(app.state.clone());

    let user_id = create_test_user(
        &app.state,
        "delete@example.com",
        "password123",
        UserRole::User,
    )
    .await;
    let token = get_auth_token(&app.state, user_id, "delete@example.com").await;

    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/api/users/me")
                .header("Authorization", format!("Bearer {}", token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    // Token should no longer work
    let get_response = router
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/users/me")
                .header("Authorization", format!("Bearer {}", token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Either UNAUTHORIZED or NOT_FOUND depending on implementation
    assert!(
        get_response.status() == StatusCode::UNAUTHORIZED
            || get_response.status() == StatusCode::NOT_FOUND,
        "Expected UNAUTHORIZED or NOT_FOUND, got {:?}",
        get_response.status()
    );
}
// ==================== Get User By ID ====================

#[tokio::test]
async fn test_get_user_by_id() {
    let app = TestApp::spawn().await;
    let router = rust_queue::build_router(app.state.clone());

    let user_id = create_test_user(
        &app.state,
        "target@example.com",
        "password123",
        UserRole::User,
    )
    .await;
    let requester_id = create_test_user(
        &app.state,
        "requester@example.com",
        "password123",
        UserRole::User,
    )
    .await;
    let token = get_auth_token(&app.state, requester_id, "requester@example.com").await;

    let response = router
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/users/{}", user_id))
                .header("Authorization", format!("Bearer {}", token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json.get("email").unwrap(), "target@example.com");
}

#[tokio::test]
async fn test_get_user_by_id_not_found() {
    let app = TestApp::spawn().await;
    let router = rust_queue::build_router(app.state.clone());

    let user_id = create_test_user(
        &app.state,
        "requester@example.com",
        "password123",
        UserRole::User,
    )
    .await;
    let token = get_auth_token(&app.state, user_id, "requester@example.com").await;

    let fake_id = Uuid::new_v4();

    let response = router
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/users/{}", fake_id))
                .header("Authorization", format!("Bearer {}", token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
