use axum::{
    body::Body,
    http::Request,
    response::Response,
};
use dotenv::dotenv;
use rust_queue::state::AppState;
use rust_queue::{models::user::UserRole, repository::WriteRepository};
use serde_json::Value;
use sqlx::{Executor, PgPool};
use tower::ServiceExt;
use uuid::Uuid;

pub struct TestApp {
    pub state: AppState,
    pub db_name: String,
    base_url: String,
}

impl TestApp {
    pub async fn spawn() -> Self {
        dotenv().ok();

        let password_pepper =
            std::env::var("PASSWORD_PEPPER").expect("PASSWORD_PEPPER must be set");
        let jwt_secret = std::env::var("JWT_SECRET").expect("JWT_SECRET must be set");

        // Base postgres url
        let base_url = "postgres://postgres:postgres@localhost:5432".to_string();

        // Create unique database name
        let db_name = format!(
            "rust_queue_test_{}",
            Uuid::new_v4().to_string().replace("-", "")
        );

        // Connect to postgres to create test database
        let admin_pool = PgPool::connect(&format!("{}/postgres", base_url))
            .await
            .expect("Failed to connect to postgres");

        // Create test database
        admin_pool
            .execute(format!(r#"CREATE DATABASE "{}""#, db_name).as_str())
            .await
            .expect("Failed to create test database");

        // Connect to test database and run migrations
        let database_url = format!("{}/{}", base_url, db_name);
        let state = AppState::new(&database_url, &password_pepper, &jwt_secret, 24, 30)
            .await
            .expect("Failed to create app state");

        Self {
            state,
            db_name,
            base_url,
        }
    }
}

impl Drop for TestApp {
    fn drop(&mut self) {
        let db_name = self.db_name.clone();
        let base_url = self.base_url.clone();

        // Spawn a blocking task to clean up. Since is a synchronous function, but our
        // cleanup needs async operations (database queries).
        // Use std::thread to run cleanup synchronously in drop
        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let admin_pool = PgPool::connect(&format!("{}/postgres", base_url))
                    .await
                    .expect("Failed to connect for cleanup");

                // Terminate connections
                admin_pool
                    .execute(
                        format!(
                            r#"SELECT pg_terminate_backend(pg_stat_activity.pid)
                            FROM pg_stat_activity
                            WHERE pg_stat_activity.datname = '{}'
                            AND pid <> pg_backend_pid()"#,
                            db_name
                        )
                        .as_str(),
                    )
                    .await
                    .ok();

                // Drop database
                admin_pool
                    .execute(format!(r#"DROP DATABASE IF EXISTS "{}""#, db_name).as_str())
                    .await
                    .ok();
            });
        })
        .join()
        .ok();
    }
}

pub async fn debug_response(response: Response) {
    let status = response.status();
    println!("Status: {:?}", status);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    println!("Body: {}", String::from_utf8_lossy(&body));
    let json: Value = serde_json::from_slice(&body).unwrap();
    println!("JSON: {}", serde_json::to_string_pretty(&json).unwrap());
}

pub async fn create_test_user(
    state: &AppState,
    email: &str,
    password: &str,
    role: UserRole,
) -> Uuid {
    use rust_queue::models::user::CreateUserRequest;

    let password_hash = state.password_service.hash(password).unwrap();

    let user = state
        .users
        .create(CreateUserRequest {
            email: email.to_string(),
            name: "Test User".to_string(),
            password: password_hash,
            role,
        })
        .await
        .expect("Failed to create test user");

    user.id
}

pub async fn get_auth_token(state: &AppState, user_id: Uuid, email: &str) -> String {
    state
        .jwt_service
        .generate_access_token(user_id, email)
        .expect("Failed to generate token")
}

/// Helper to create a test user and return (user_id, auth_token).
/// Saves boilerplate in tests that just need an authenticated user.
pub async fn create_authenticated_user(state: &AppState) -> (Uuid, String) {
    let user_id = create_test_user(state, "testuser@example.com", "password123", UserRole::User).await;
    let token = get_auth_token(state, user_id, "testuser@example.com").await;
    (user_id, token)
}

/// Helper to make an authenticated request and return the response.
pub async fn auth_request(
    state: &AppState,
    token: &str,
    method: &str,
    uri: &str,
    body: Option<serde_json::Value>,
) -> Response {
    let router = rust_queue::build_router(state.clone());

    let mut builder = Request::builder()
        .method(method)
        .uri(uri)
        .header("Authorization", format!("Bearer {}", token));

    let body = if let Some(json_body) = body {
        builder = builder.header("Content-Type", "application/json");
        Body::from(serde_json::to_string(&json_body).unwrap())
    } else {
        Body::empty()
    };

    router
        .oneshot(builder.body(body).unwrap())
        .await
        .unwrap()
}

/// Helper to extract JSON body from a response.
pub async fn response_json(response: Response) -> Value {
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&body).unwrap()
}
