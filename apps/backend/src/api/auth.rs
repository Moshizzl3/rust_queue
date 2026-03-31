use axum::{
    Json, Router,
    extract::State,
    http::{StatusCode, header},
    response::{AppendHeaders, IntoResponse},
    routing::post,
};
use cookie::{Cookie, SameSite};

use crate::models::auth::{LoginRequest, LoginResponse, RefreshResponse, RegisterResponse};
use crate::models::user::{CreateUserRequest, UserResponse};
use crate::repository::{FilterParams, ReadRepository, WriteRepository};
use crate::state::AppState;
use crate::validation::validate;
use crate::{error::AppError, models::responses::DataResponse};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/register", post(register))
        .route("/login", post(login))
        .route("/refresh", post(refresh))
        .route("/logout", post(logout))
}

// Helper to build cookies
fn build_access_cookie(token: &str, max_age_mins: i64) -> Cookie<'static> {
    Cookie::build(("access_token", token.to_string()))
        .http_only(true)
        .secure(true)
        .same_site(SameSite::Lax)
        .path("/")
        .max_age(cookie::time::Duration::minutes(max_age_mins))
        .build()
}

fn build_refresh_cookie(token: &str, max_age_days: i64) -> Cookie<'static> {
    Cookie::build(("refresh_token", token.to_string()))
        .http_only(true)
        .secure(true)
        .same_site(SameSite::Strict) // Stricter for refresh token
        .path("/api/auth") // Only sent to auth endpoints
        .max_age(cookie::time::Duration::days(max_age_days))
        .build()
}

fn build_expired_cookie(name: &str, path: &str) -> Cookie<'static> {
    Cookie::build((name.to_string(), ""))
        .http_only(true)
        .secure(true)
        .same_site(SameSite::Lax)
        .path(path.to_string())
        .max_age(cookie::time::Duration::seconds(0))
        .build()
}

fn extract_cookie_value<'a>(cookie_header: &'a str, name: &str) -> Option<&'a str> {
    cookie_header.split(';').find_map(|cookie| {
        let cookie = cookie.trim();
        cookie.strip_prefix(&format!("{}=", name))
    })
}

#[utoipa::path(
    post,
    path = "/api/auth/register",
    request_body = CreateUserRequest,
    responses(
        (status = 201, description = "User created", body = DataResponse<RegisterResponse>),
        (status = 409, description = "Email already exists"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Auth"
)]
pub async fn register(
    State(state): State<AppState>,
    Json(mut payload): Json<CreateUserRequest>,
) -> Result<impl IntoResponse, AppError> {
    validate(&payload)?;
    payload.password = state.password_service.hash(&payload.password)?;

    let user = state.users.create(payload).await?;

    let (access_token, refresh_token) = state
        .jwt_service
        .generate_token_pair(user.id, &user.email)
        .map_err(|e| AppError::InternalError(e.to_string()))?;

    let access_cookie = build_access_cookie(&access_token, state.jwt_service.access_expiry_mins());
    let refresh_cookie =
        build_refresh_cookie(&refresh_token, state.jwt_service.refresh_expiry_days());
    let headers = AppendHeaders([
        (header::SET_COOKIE, access_cookie.to_string()),
        (header::SET_COOKIE, refresh_cookie.to_string()),
    ]);

    Ok((
        StatusCode::CREATED,
        headers,
        Json(DataResponse::new(RegisterResponse {
            access_token,
            user: UserResponse::from(user),
        })),
    ))
}

#[utoipa::path(
    post,
    path = "/api/auth/login",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "Login successful", body = DataResponse<LoginResponse>),
        (status = 401, description = "Invalid credentials")
    ),
    tag = "Auth"
)]
pub async fn login(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> Result<impl IntoResponse, AppError> {
    validate(&payload)?;

    let user = state
        .users
        .find_one(&FilterParams::new().add_string("email", &payload.email))
        .await?
        .ok_or_else(|| AppError::Unauthorized("Invalid credentials".to_string()))?;

    let is_valid = state
        .password_service
        .verify(&payload.password, &user.password_hash)?;

    if !is_valid {
        return Err(AppError::Unauthorized("Invalid credentials".to_string()));
    }

    let (access_token, refresh_token) = state
        .jwt_service
        .generate_token_pair(user.id, &user.email)
        .map_err(|e| AppError::InternalError(format!("Token generation failed: {}", e)))?;

    let access_cookie = build_access_cookie(&access_token, state.jwt_service.access_expiry_mins());
    let refresh_cookie =
        build_refresh_cookie(&refresh_token, state.jwt_service.refresh_expiry_days());
    let headers = AppendHeaders([
        (header::SET_COOKIE, access_cookie.to_string()),
        (header::SET_COOKIE, refresh_cookie.to_string()),
    ]);

    Ok((
        headers,
        Json(DataResponse::new(LoginResponse {
            access_token,
            user_id: user.id,
            email: user.email,
        })),
    ))
}

#[utoipa::path(
    post,
    path = "/api/auth/refresh",
    responses(
        (status = 200, description = "Tokens refreshed", body = DataResponse<RefreshResponse>),
        (status = 401, description = "Invalid refresh token")
    ),
    tag = "Auth"
)]
pub async fn refresh(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> Result<impl IntoResponse, AppError> {
    // Extract refresh token from cookie
    let cookie_header = headers
        .get(header::COOKIE)
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| AppError::Unauthorized("Missing refresh token".to_string()))?;

    let refresh_token = extract_cookie_value(cookie_header, "refresh_token")
        .ok_or_else(|| AppError::Unauthorized("Missing refresh token".to_string()))?;

    // Validate the refresh token
    let token_data = state
        .jwt_service
        .validate_refresh_token(refresh_token)
        .map_err(|_| AppError::Unauthorized("Invalid or expired refresh token".to_string()))?;

    let user_id = token_data.claims.sub;

    // Fetch user to get email (and verify user still exists)
    let user = state
        .users
        .find_by_id(user_id)
        .await?
        .ok_or_else(|| AppError::Unauthorized("User not found".to_string()))?;

    // Generate new token pair (token rotation)
    let (access_token, new_refresh_token) = state
        .jwt_service
        .generate_token_pair(user.id, &user.email)
        .map_err(|e| AppError::InternalError(format!("Token generation failed: {}", e)))?;

    let access_cookie = build_access_cookie(&access_token, state.jwt_service.access_expiry_mins());
    let refresh_cookie =
        build_refresh_cookie(&new_refresh_token, state.jwt_service.refresh_expiry_days());
    let headers = AppendHeaders([
        (header::SET_COOKIE, access_cookie.to_string()),
        (header::SET_COOKIE, refresh_cookie.to_string()),
    ]);

    Ok((
        headers,
        Json(DataResponse::new(RefreshResponse { access_token })),
    ))
}

#[utoipa::path(
    post,
    path = "/api/auth/logout",
    responses(
        (status = 200, description = "Logged out successfully")
    ),
    tag = "Auth"
)]
pub async fn logout() -> impl IntoResponse {
    let expired_access = build_expired_cookie("access_token", "/");
    let expired_refresh = build_expired_cookie("refresh_token", "/api/auth");
    let headers = AppendHeaders([
        (header::SET_COOKIE, expired_access.to_string()),
        (header::SET_COOKIE, expired_refresh.to_string()),
    ]);

    (headers, StatusCode::OK)
}
