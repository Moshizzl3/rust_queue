use crate::api;
use crate::error;
use crate::models;

use utoipa::{
    Modify, OpenApi,
    openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme},
};

struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let components = openapi.components.as_mut().unwrap();
        components.add_security_scheme(
            "bearer_auth",
            SecurityScheme::Http(
                HttpBuilder::new()
                    .scheme(HttpAuthScheme::Bearer)
                    .bearer_format("JWT")
                    .build(),
            ),
        );
    }
}

#[derive(OpenApi)]
#[openapi(
    paths(
        // Health
        api::health::get_health_status,

        // Auth
        api::auth::login,
        api::auth::register,
        api::auth::refresh,
        api::auth::logout,

        // Users
        api::user::get_current_user,
        api::user::update_current_user,
        api::user::delete_current_user,
        api::user::get_users,
        api::user::get_user_by_id,
        api::user::update_user,
        api::user::delete_user,

        // Jobs
        api::job::create_job,
        api::job::get_job,
        api::job::list_jobs,
        api::job::cancel_job,
        api::job::get_stats,
    ),
    components(
        schemas(
            // Health
            models::health::Health,

            // Auth
            models::auth::LoginRequest,
            models::auth::LoginResponse,
            models::auth::RegisterResponse,
            models::auth::RefreshResponse,

            // Users
            models::user::UserResponse,
            models::user::CreateUserRequest,
            models::user::UpdateUserRequest,

            // Jobs
            models::job::JobStatus,
            models::job::CreateJobRequest,
            models::job::JobResponse,
            models::job::JobDetailResponse,
            models::job::JobStats,

            // Pagination
            models::pagination::SortOrder,
            models::pagination::PaginationMetadata,

            // Error
            error::ErrorResponse,

            // Responses
            models::responses::EmptyResponse,
        )
    ),
    modifiers(&SecurityAddon),
    tags(
        (name = "Health",       description = "Health check endpoints"),
        (name = "Auth",         description = "Authentication endpoints"),
        (name = "Users",        description = "User management"),
        (name = "Jobs",         description = "Job queue management"),
    ),
    info(
        title = "Rust Queue API",
        version = "0.1.0",
        description = "Distributed task queue API with dashboard"
    )
)]
pub struct ApiDoc;
