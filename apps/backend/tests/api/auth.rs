use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use rust_queue::models::user::UserRole;
use serde_json::{Value, json};
use tower::ServiceExt;

use crate::common::helpers::{TestApp, create_test_user};

// ==================== Database Isolation ====================

#[tokio::test]
async fn test_fresh_database() {
    let app = TestApp::spawn().await;

    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
        .fetch_one(&app.state.pool)
        .await
        .unwrap();

    assert_eq!(count.0, 0, "Database should be empty at start of test");

    create_test_user(
        &app.state,
        "test@example.com",
        "password123",
        UserRole::User,
    )
    .await;

    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
        .fetch_one(&app.state.pool)
        .await
        .unwrap();

    assert_eq!(count.0, 1);

    println!("Auth test database: {}", app.db_name);
}

// ==================== Register ====================

#[tokio::test]
async fn test_register_success() {
    let app = TestApp::spawn().await;
    let router = rust_queue::build_router(app.state.clone());

    let body = json!({
        "email": "newuser@example.com",
        "name": "New User",
        "password": "password123",
        "role": "user"
    });

    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/register")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    let auth_data = json.get("data").unwrap();

    assert!(auth_data.get("access_token").is_some());
}

#[tokio::test]
async fn test_register_invalid_email() {
    let app = TestApp::spawn().await;
    let router = rust_queue::build_router(app.state.clone());

    let body = json!({
        "email": "not-an-email",
        "name": "Test User",
        "password": "password123",
        "role": "user"
    });

    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/register")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_register_empty_email() {
    let app = TestApp::spawn().await;
    let router = rust_queue::build_router(app.state.clone());

    let body = json!({
        "email": "",
        "name": "Test User",
        "password": "password123",
        "role": "user"
    });

    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/register")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_register_short_password() {
    let app = TestApp::spawn().await;
    let router = rust_queue::build_router(app.state.clone());

    let body = json!({
        "email": "test@example.com",
        "name": "Test User",
        "password": "123",
        "role": "user"
    });

    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/register")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_register_empty_password() {
    let app = TestApp::spawn().await;
    let router = rust_queue::build_router(app.state.clone());

    let body = json!({
        "email": "test@example.com",
        "name": "Test User",
        "password": "",
        "role": "user"
    });

    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/register")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_register_short_username() {
    let app = TestApp::spawn().await;
    let router = rust_queue::build_router(app.state.clone());

    let body = json!({
        "email": "test@example.com",
        "name": "A",
        "password": "password123",
        "role": "user"
    });

    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/register")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_register_duplicate_email() {
    let app = TestApp::spawn().await;
    let router = rust_queue::build_router(app.state.clone());

    create_test_user(
        &app.state,
        "existing@example.com",
        "password123",
        UserRole::User,
    )
    .await;

    let body = json!({
        "email": "existing@example.com",
        "name": "Another User",
        "password": "password123",
        "role": "user"
    });

    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/register")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn test_register_missing_fields() {
    let app = TestApp::spawn().await;
    let router = rust_queue::build_router(app.state.clone());

    let body = json!({
        "email": "test@example.com"
    });

    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/register")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn test_register_invalid_json() {
    let app = TestApp::spawn().await;
    let router = rust_queue::build_router(app.state.clone());

    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/register")
                .header("Content-Type", "application/json")
                .body(Body::from("not valid json"))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_register_email_case_insensitive() {
    let app = TestApp::spawn().await;
    let router = rust_queue::build_router(app.state.clone());

    // Register with lowercase
    create_test_user(
        &app.state,
        "test@example.com",
        "password123",
        UserRole::User,
    )
    .await;

    // Try to register with uppercase - should conflict
    let body = json!({
        "email": "TEST@EXAMPLE.COM",
        "name": "Another User",
        "password": "password123"
    });

    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/register")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CONFLICT);
}

// ==================== Login ====================

#[tokio::test]
async fn test_login_success() {
    let app = TestApp::spawn().await;
    let router = rust_queue::build_router(app.state.clone());

    create_test_user(
        &app.state,
        "login@example.com",
        "password123",
        UserRole::User,
    )
    .await;

    let body = json!({
        "email": "login@example.com",
        "password": "password123"
    });

    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/login")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    let auth_data = json.get("data").unwrap();

    assert!(auth_data.get("access_token").is_some());
    assert_eq!(auth_data.get("email").unwrap(), "login@example.com");
}

#[tokio::test]
async fn test_login_wrong_password() {
    let app = TestApp::spawn().await;
    let router = rust_queue::build_router(app.state.clone());

    create_test_user(
        &app.state,
        "test@example.com",
        "password123",
        UserRole::User,
    )
    .await;

    let body = json!({
        "email": "test@example.com",
        "password": "wrongpassword"
    });

    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/login")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_login_nonexistent_user() {
    let app = TestApp::spawn().await;
    let router = rust_queue::build_router(app.state.clone());

    let body = json!({
        "email": "nonexistent@example.com",
        "password": "password123"
    });

    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/login")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_login_invalid_email_format() {
    let app = TestApp::spawn().await;
    let router = rust_queue::build_router(app.state.clone());

    let body = json!({
        "email": "not-an-email",
        "password": "password123"
    });

    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/login")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_login_empty_password() {
    let app = TestApp::spawn().await;
    let router = rust_queue::build_router(app.state.clone());

    let body = json!({
        "email": "test@example.com",
        "password": ""
    });

    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/login")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_login_missing_fields() {
    let app = TestApp::spawn().await;
    let router = rust_queue::build_router(app.state.clone());

    let body = json!({
        "email": "test@example.com"
    });

    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/login")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn test_login_token_is_valid_jwt() {
    let app = TestApp::spawn().await;
    let router = rust_queue::build_router(app.state.clone());

    create_test_user(
        &app.state,
        "test@example.com",
        "password123",
        UserRole::User,
    )
    .await;

    let body = json!({
        "email": "test@example.com",
        "password": "password123"
    });

    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/login")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    let auth_data = json.get("data").unwrap();

    let token = auth_data.get("access_token").unwrap().as_str().unwrap();

    // JWT has 3 parts separated by dots
    let parts: Vec<&str> = token.split('.').collect();
    assert_eq!(parts.len(), 3, "Token should be a valid JWT with 3 parts");
}

// ==================== Refresh Token ====================

#[tokio::test]
async fn test_login_sets_cookies() {
    let app = TestApp::spawn().await;
    let router = rust_queue::build_router(app.state.clone());

    create_test_user(
        &app.state,
        "test@example.com",
        "password123",
        UserRole::User,
    )
    .await;

    let body = json!({
        "email": "test@example.com",
        "password": "password123"
    });

    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/login")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let cookies: Vec<_> = response.headers().get_all("set-cookie").iter().collect();

    assert_eq!(
        cookies.len(),
        2,
        "Should set both access_token and refresh_token cookies"
    );

    let cookie_str: String = cookies
        .iter()
        .map(|c| c.to_str().unwrap())
        .collect::<Vec<_>>()
        .join("; ");

    assert!(
        cookie_str.contains("access_token="),
        "Should set access_token cookie"
    );
    assert!(
        cookie_str.contains("refresh_token="),
        "Should set refresh_token cookie"
    );
    assert!(
        cookie_str.contains("HttpOnly"),
        "Cookies should be HttpOnly"
    );
}

#[tokio::test]
async fn test_refresh_token_success() {
    let app = TestApp::spawn().await;

    create_test_user(
        &app.state,
        "test@example.com",
        "password123",
        UserRole::User,
    )
    .await;

    // First, login to get tokens
    let login_body = json!({
        "email": "test@example.com",
        "password": "password123"
    });

    let router = rust_queue::build_router(app.state.clone());
    let login_response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/login")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&login_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Extract refresh token from cookies
    let cookies: Vec<_> = login_response
        .headers()
        .get_all("set-cookie")
        .iter()
        .map(|c| c.to_str().unwrap().to_string())
        .collect();

    let refresh_cookie = cookies
        .iter()
        .find(|c| c.starts_with("refresh_token="))
        .expect("Should have refresh_token cookie");

    // Extract just the cookie value for the Cookie header
    let refresh_token = refresh_cookie.split(';').next().unwrap();

    // Now call refresh endpoint
    let router = rust_queue::build_router(app.state.clone());
    let refresh_response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/refresh")
                .header("Cookie", refresh_token)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(refresh_response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(refresh_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    let auth_data = json.get("data").unwrap();

    assert!(
        auth_data.get("access_token").is_some(),
        "Should return new access_token"
    );
}

#[tokio::test]
async fn test_refresh_token_missing() {
    let app = TestApp::spawn().await;
    let router = rust_queue::build_router(app.state.clone());

    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/refresh")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_refresh_token_invalid() {
    let app = TestApp::spawn().await;
    let router = rust_queue::build_router(app.state.clone());

    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/refresh")
                .header("Cookie", "refresh_token=invalid.token.here")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_refresh_token_cannot_use_access_token() {
    let app = TestApp::spawn().await;

    create_test_user(
        &app.state,
        "test@example.com",
        "password123",
        UserRole::User,
    )
    .await;

    // Login to get tokens
    let login_body = json!({
        "email": "test@example.com",
        "password": "password123"
    });

    let router = rust_queue::build_router(app.state.clone());
    let login_response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/login")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&login_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Extract access token from cookies
    let cookies: Vec<_> = login_response
        .headers()
        .get_all("set-cookie")
        .iter()
        .map(|c| c.to_str().unwrap().to_string())
        .collect();

    let access_cookie = cookies
        .iter()
        .find(|c| c.starts_with("access_token="))
        .expect("Should have access_token cookie");

    let access_token_value = access_cookie
        .split(';')
        .next()
        .unwrap()
        .strip_prefix("access_token=")
        .unwrap();

    // Try to use access token as refresh token - should fail
    let router = rust_queue::build_router(app.state.clone());
    let refresh_response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/refresh")
                .header("Cookie", format!("refresh_token={}", access_token_value))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(refresh_response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_refresh_rotates_tokens() {
    let app = TestApp::spawn().await;

    create_test_user(
        &app.state,
        "test@example.com",
        "password123",
        UserRole::User,
    )
    .await;

    // Login
    let login_body = json!({
        "email": "test@example.com",
        "password": "password123"
    });

    let router = rust_queue::build_router(app.state.clone());
    let login_response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/login")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&login_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    let login_cookies: Vec<_> = login_response
        .headers()
        .get_all("set-cookie")
        .iter()
        .map(|c| c.to_str().unwrap().to_string())
        .collect();

    let original_refresh = login_cookies
        .iter()
        .find(|c| c.starts_with("refresh_token="))
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_string();

    // Refresh
    let router = rust_queue::build_router(app.state.clone());
    let refresh_response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/refresh")
                .header("Cookie", &original_refresh)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let refresh_cookies: Vec<_> = refresh_response
        .headers()
        .get_all("set-cookie")
        .iter()
        .map(|c| c.to_str().unwrap().to_string())
        .collect();

    let new_refresh = refresh_cookies
        .iter()
        .find(|c| c.starts_with("refresh_token="))
        .unwrap()
        .split(';')
        .next()
        .unwrap();

    assert_ne!(
        original_refresh, new_refresh,
        "Refresh token should be rotated"
    );
}

// ==================== Logout ====================

#[tokio::test]
async fn test_logout_clears_cookies() {
    let app = TestApp::spawn().await;
    let router = rust_queue::build_router(app.state.clone());

    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/logout")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let cookies: Vec<_> = response
        .headers()
        .get_all("set-cookie")
        .iter()
        .map(|c| c.to_str().unwrap())
        .collect();

    assert_eq!(cookies.len(), 2, "Should clear both cookies");

    // Check that cookies are expired (Max-Age=0)
    for cookie in cookies {
        assert!(
            cookie.contains("Max-Age=0") || cookie.contains("max-age=0"),
            "Cookie should be expired: {}",
            cookie
        );
    }
}

// ==================== Protected Routes with Cookies ====================

#[tokio::test]
async fn test_protected_route_with_cookie() {
    let app = TestApp::spawn().await;

    create_test_user(
        &app.state,
        "test@example.com",
        "password123",
        UserRole::User,
    )
    .await;

    // Login
    let login_body = json!({
        "email": "test@example.com",
        "password": "password123"
    });

    let router = rust_queue::build_router(app.state.clone());
    let login_response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/login")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&login_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    let cookies: Vec<_> = login_response
        .headers()
        .get_all("set-cookie")
        .iter()
        .map(|c| c.to_str().unwrap().to_string())
        .collect();

    let access_cookie = cookies
        .iter()
        .find(|c| c.starts_with("access_token="))
        .unwrap()
        .split(';')
        .next()
        .unwrap();

    // Access protected route with cookie
    let router = rust_queue::build_router(app.state.clone());
    let response = router
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/users/me")
                .header("Cookie", access_cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_protected_route_with_bearer_token() {
    let app = TestApp::spawn().await;

    create_test_user(
        &app.state,
        "test@example.com",
        "password123",
        UserRole::User,
    )
    .await;

    // Login
    let login_body = json!({
        "email": "test@example.com",
        "password": "password123"
    });

    let router = rust_queue::build_router(app.state.clone());
    let login_response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/login")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&login_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    let body = axum::body::to_bytes(login_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    let auth_data = json.get("data").unwrap();
    let token = auth_data.get("access_token").unwrap().as_str().unwrap();

    // Access protected route with Bearer token (for Swagger compatibility)
    let router = rust_queue::build_router(app.state.clone());
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
}

#[tokio::test]
async fn test_protected_route_no_auth() {
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
async fn test_protected_route_expired_token() {
    let app = TestApp::spawn().await;
    let router = rust_queue::build_router(app.state.clone());

    // Create an expired token manually
    let expired_token = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIwMDAwMDAwMC0wMDAwLTAwMDAtMDAwMC0wMDAwMDAwMDAwMDAiLCJlbWFpbCI6InRlc3RAZXhhbXBsZS5jb20iLCJleHAiOjEwMDAwMDAwMDAsImlhdCI6MTAwMDAwMDAwMCwidG9rZW5fdHlwZSI6ImFjY2VzcyJ9.invalid";

    let response = router
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/users/me")
                .header("Authorization", format!("Bearer {}", expired_token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_protected_route_cookie_takes_precedence() {
    let app = TestApp::spawn().await;

    create_test_user(
        &app.state,
        "test@example.com",
        "password123",
        UserRole::User,
    )
    .await;

    // Login to get valid cookie
    let login_body = json!({
        "email": "test@example.com",
        "password": "password123"
    });

    let router = rust_queue::build_router(app.state.clone());
    let login_response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/login")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&login_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    let cookies: Vec<_> = login_response
        .headers()
        .get_all("set-cookie")
        .iter()
        .map(|c| c.to_str().unwrap().to_string())
        .collect();

    let access_cookie = cookies
        .iter()
        .find(|c| c.starts_with("access_token="))
        .unwrap()
        .split(';')
        .next()
        .unwrap();

    // Send both cookie (valid) and Bearer (invalid) - cookie should be used
    let router = rust_queue::build_router(app.state.clone());
    let response = router
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/users/me")
                .header("Cookie", access_cookie)
                .header("Authorization", "Bearer invalid.token.here")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

// ==================== Token Type Validation ====================

#[tokio::test]
async fn test_cannot_use_refresh_token_as_access_token() {
    let app = TestApp::spawn().await;

    create_test_user(
        &app.state,
        "test@example.com",
        "password123",
        UserRole::User,
    )
    .await;

    // Login
    let login_body = json!({
        "email": "test@example.com",
        "password": "password123"
    });

    let router = rust_queue::build_router(app.state.clone());
    let login_response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/login")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&login_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    let cookies: Vec<_> = login_response
        .headers()
        .get_all("set-cookie")
        .iter()
        .map(|c| c.to_str().unwrap().to_string())
        .collect();

    // Extract refresh token value
    let refresh_token = cookies
        .iter()
        .find(|c| c.starts_with("refresh_token="))
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .strip_prefix("refresh_token=")
        .unwrap();

    // Try to use refresh token as access token - should fail
    let router = rust_queue::build_router(app.state.clone());
    let response = router
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/users/me")
                .header("Cookie", format!("access_token={}", refresh_token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}
