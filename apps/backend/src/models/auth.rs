use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

use crate::models::serde_helpers::lowercase;
use crate::models::user::UserResponse;

#[derive(Debug, Deserialize, ToSchema, Validate)]
pub struct LoginRequest {
    #[validate(email(message = "Invalid email format"))]
    #[serde(deserialize_with = "lowercase")]
    #[schema(example = "user@example.com")]
    pub email: String,
    #[validate(length(min = 1, message = "Password is required"))]
    #[schema(example = "password123")]
    pub password: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct LoginResponse {
    pub access_token: String,
    pub user_id: uuid::Uuid,
    pub email: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct RegisterResponse {
    pub access_token: String,
    pub user: UserResponse,
}

#[derive(Debug, Clone)]
pub struct AuthUser {
    pub user_id: Uuid,
    pub email: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct RefreshResponse {
    pub access_token: String,
}
