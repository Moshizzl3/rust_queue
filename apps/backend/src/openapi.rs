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
    ),
    info(
        title = "Rust queue API",
        version = "0.1.0",
        description = "Rust queue -  API for tracking queue."
    )
)]
pub struct ApiDoc;
