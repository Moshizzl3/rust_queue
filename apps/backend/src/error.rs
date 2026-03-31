use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;
use tracing::error;
use utoipa::ToSchema;
use validator::ValidationErrors;

use crate::repository::FilterError;

#[derive(Debug)]
pub enum AppError {
    // 400
    BadRequest(String),

    // 401
    Unauthorized(String),

    // 403
    Forbidden(String),

    // 404
    NotFound(String),

    // 409
    Conflict(String),

    // 500
    InternalError(String),

    // Database errors
    Database(sqlx::Error),

    // Validation
    Validation(ValidationErrors),
}

#[derive(Serialize, ToSchema)]
pub struct ErrorResponse {
    #[schema(example = "Bad Request")]
    pub error: String,
    #[schema(example = "Invalid email format")]
    pub message: String,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_type, message) = match self {
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, "Bad Request", msg),
            AppError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, "Unauthorized", msg),
            AppError::Forbidden(msg) => (StatusCode::FORBIDDEN, "Forbidden", msg),
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, "Not Found", msg),
            AppError::Conflict(msg) => (StatusCode::CONFLICT, "Conflict", msg),
            AppError::InternalError(msg) => {
                error!("Internal error: {}", msg);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal Server Error",
                    msg,
                )
            }
            AppError::Database(err) => {
                error!("Database error: {:?}", err);

                if let sqlx::Error::Database(db_err) = &err {
                    if db_err.is_unique_violation() {
                        return (
                            StatusCode::CONFLICT,
                            Json(ErrorResponse {
                                error: "Conflict".to_string(),
                                message: "A record with this value already exists".to_string(),
                            }),
                        )
                            .into_response();
                    }
                    // Check for partially created stuff in db
                    else if db_err.is_foreign_key_violation() {
                        return (
                            StatusCode::BAD_REQUEST,
                            Json(ErrorResponse {
                                error: "Bad Request".to_string(),
                                message: "Something missing in the database.".to_string(),
                            }),
                        )
                            .into_response();
                    }
                }

                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal Server Error",
                    "Database error".to_string(),
                )
            }
            AppError::Validation(errors) => {
                let messages: Vec<String> = errors
                    .field_errors()
                    .iter()
                    .flat_map(|(field, errs)| {
                        errs.iter().map(move |e| {
                            format!(
                                "{}: {}",
                                field,
                                e.message
                                    .as_ref()
                                    .unwrap_or(&std::borrow::Cow::Borrowed("invalid"))
                            )
                        })
                    })
                    .collect();

                (
                    StatusCode::BAD_REQUEST,
                    "Validation Error",
                    messages.join(", "),
                )
            }
        };

        (
            status,
            Json(ErrorResponse {
                error: error_type.to_string(),
                message,
            }),
        )
            .into_response()
    }
}

impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        AppError::Database(err)
    }
}

impl From<argon2::password_hash::Error> for AppError {
    fn from(err: argon2::password_hash::Error) -> Self {
        AppError::InternalError(format!("Password hashing error: {}", err))
    }
}

impl From<ValidationErrors> for AppError {
    fn from(err: ValidationErrors) -> Self {
        AppError::Validation(err)
    }
}

impl From<FilterError> for AppError {
    fn from(err: FilterError) -> Self {
        match err {
            FilterError::InvalidField(field, allowed) => AppError::BadRequest(format!(
                "Invalid filter field: {}. Allowed: {}",
                field, allowed
            )),
            FilterError::Database(db_err) => AppError::Database(db_err),
        }
    }
}
