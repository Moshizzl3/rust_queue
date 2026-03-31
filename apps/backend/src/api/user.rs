use crate::error::AppError;
use crate::models::auth::AuthUser;
use crate::models::pagination::{PaginatedResponse, PaginationParams};
use crate::models::responses::{DataResponse, EmptyResponse};
use crate::models::user::UpdateUserRequest;
use crate::repository::{ReadRepository, WriteRepository};
use crate::validation::validate;
use crate::{models::user::UserResponse, state::AppState};
use axum::extract::Query;
use axum::{
    Extension, Json, Router,
    extract::{Path, State},
    response::IntoResponse,
    routing::get,
};
use uuid::Uuid;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(get_users))
        .route(
            "/me",
            get(get_current_user)
                .patch(update_current_user)
                .delete(delete_current_user),
        )
        .route(
            "/{id}",
            get(get_user_by_id).delete(delete_user).patch(update_user),
        )
}

#[utoipa::path(
    get,
    path = "/api/users/me",
    responses(
        (status = 200, description = "Returns current user.", body = DataResponse<UserResponse>),
        (status = 401, description = "Unauthorized")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Users"
)]
async fn get_current_user(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthUser>,
) -> Result<impl IntoResponse, AppError> {
    let user = state
        .users
        .find_by_id(auth_user.user_id)
        .await?
        .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

    Ok(Json(DataResponse::new(UserResponse::from(user))))
}

#[utoipa::path(
    patch,
    path = "/api/users/me",
    request_body = UpdateUserRequest,
    responses(
        (status = 200, description = "User updated.", body = DataResponse<UserResponse>),
        (status = 401, description = "Unauthorized")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Users"
)]
async fn update_current_user(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthUser>,
    Json(payload): Json<UpdateUserRequest>,
) -> Result<impl IntoResponse, AppError> {
    validate(&payload)?;
    let updated = state
        .users
        .update(auth_user.user_id, payload)
        .await?
        .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

    Ok(Json(DataResponse::new(UserResponse::from(updated))))
}

#[utoipa::path(
    delete,
    path = "/api/users/me",
    responses(
        (status = 200, description = "User deleted."),
        (status = 401, description = "Unauthorized")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Users"
)]
async fn delete_current_user(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthUser>,
) -> Result<impl IntoResponse, AppError> {
    state.users.delete(auth_user.user_id).await?;

    Ok(Json(EmptyResponse::default()))
}

#[utoipa::path(
    get,
    path = "/api/users/{id}",
    responses(
        (status = 200, description = "Returns a user.", body = DataResponse<UserResponse>),
        (status = 404, description = "User not found.")
    ),
    params(
        ("id" = Uuid, Path, description = "User ID")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Users"
)]
async fn get_user_by_id(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let user = state
        .users
        .find_by_id(id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("User {} not found", id)))?;

    Ok(Json(UserResponse::from(user)))
}

#[utoipa::path(
    get,
    path = "/api/users",
    params(PaginationParams),
    responses(
        (status = 200, description = "Returns paginated list of users.")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Users"
)]
async fn get_users(
    State(state): State<AppState>,
    Query(pagination): Query<PaginationParams>,
) -> Result<impl IntoResponse, AppError> {
    let result = state.users.find_all(&pagination).await?;

    let response = PaginatedResponse::new(
        result.data.into_iter().map(UserResponse::from).collect(),
        result.total,
        &pagination,
    );

    Ok(Json(response))
}

#[utoipa::path(
    delete,
    path = "/api/users/{id}",
    responses(
        (status = 200, description = "User deleted."),
        (status = 404, description = "User not found.")
    ),
    params(
        ("id" = Uuid, Path, description = "User ID")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Users"
)]
async fn delete_user(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let deleted = state.users.delete(id).await?;

    if deleted {
        Ok(Json(EmptyResponse::default()))
    } else {
        Err(AppError::NotFound(format!("User {} not found", id)))
    }
}

#[utoipa::path(
    patch,
    path = "/api/users/{id}",
    request_body = UpdateUserRequest,
    responses(
        (status = 200, description = "User updated.", body = DataResponse<UserResponse>),
        (status = 404, description = "User not found.")
    ),
    params(
        ("id" = Uuid, Path, description = "User ID")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Users"
)]
async fn update_user(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateUserRequest>,
) -> Result<impl IntoResponse, AppError> {
    validate(&payload)?;
    let updated = state
        .users
        .update(id, payload)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("User {} not found", id)))?;

    Ok(Json(DataResponse::new(UserResponse::from(updated))))
}
