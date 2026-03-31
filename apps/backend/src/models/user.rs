use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

use crate::models::serde_helpers::{lowercase, lowercase_option};

// ── Role enum ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, sqlx::Type)]
#[sqlx(type_name = "user_role", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum UserRole {
    Admin,
    User,
}

impl Default for UserRole {
    fn default() -> Self {
        Self::User
    }
}

// Entity
#[derive(Debug, Clone, FromRow, Serialize, ToSchema)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub name: String,
    pub role: UserRole,
    #[serde(skip_serializing)] // skips password
    pub password_hash: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// API responses
#[derive(Debug, Serialize, ToSchema)]
pub struct UserResponse {
    pub id: Uuid,
    pub email: String,
    pub name: String,
    pub created_at: DateTime<Utc>,
}

impl From<User> for UserResponse {
    fn from(user: User) -> Self {
        Self {
            id: user.id,
            email: user.email,
            name: user.name,
            created_at: user.created_at,
        }
    }
}

// API requests
#[derive(Debug, Deserialize, ToSchema, Validate)]
pub struct CreateUserRequest {
    #[validate(email(message = "Invalid Email format"))]
    #[schema(example = "user@example.com")]
    #[serde(deserialize_with = "lowercase")]
    pub email: String,
    #[validate(length(min = 2, max = 50, message = "Username must be 2-50 characters"))]
    #[schema(example = "John Doe")]
    pub name: String,
    #[validate(length(min = 8, message = "Password must be at least 8 characters"))]
    #[schema(example = "password123")]
    pub password: String,
    #[serde(default)]
    pub role: UserRole,
}

#[derive(Debug, Deserialize, ToSchema, Validate)]
pub struct UpdateUserRequest {
    #[validate(email(message = "Invalid Email format"))]
    #[serde(default, deserialize_with = "lowercase_option")]
    #[schema(example = "user@example.com")]
    pub email: Option<String>,
    #[validate(length(min = 2, max = 50, message = "Username must be 2-50 characters"))]
    #[schema(example = "John Doe")]
    pub name: Option<String>,
    pub role: Option<UserRole>,
}
