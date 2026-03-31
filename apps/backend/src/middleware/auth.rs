use axum::{
    extract::{Request, State},
    http::header,
    middleware::Next,
    response::Response,
};

use crate::{error::AppError, models::auth::AuthUser, state::AppState};

pub async fn auth_middleware(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Result<Response, AppError> {
    // Try cookie first, then fall back to Bearer token (for Swagger/API clients)
    let token = extract_token_from_cookie(&request)
        .or_else(|| extract_token_from_header(&request))
        .ok_or_else(|| AppError::Unauthorized("Missing authentication token".to_string()))?;

    let token_data = state
        .jwt_service
        .validate_access_token(&token)
        .map_err(|_| AppError::Unauthorized("Invalid or expired token".to_string()))?;

    let auth_user = AuthUser {
        user_id: token_data.claims.sub,
        email: token_data.claims.email,
    };

    request.extensions_mut().insert(auth_user);

    Ok(next.run(request).await)
}

fn extract_token_from_cookie(request: &Request) -> Option<String> {
    request
        .headers()
        .get(header::COOKIE)?
        .to_str()
        .ok()?
        .split(';')
        .find_map(|cookie| {
            let cookie = cookie.trim();
            cookie.strip_prefix("access_token=").map(|t| t.to_string())
        })
}

fn extract_token_from_header(request: &Request) -> Option<String> {
    request
        .headers()
        .get(header::AUTHORIZATION)?
        .to_str()
        .ok()?
        .strip_prefix("Bearer ")
        .map(|t| t.to_string())
}
