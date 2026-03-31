#[macro_use]
pub mod macros;
pub mod api;
pub mod config;
pub mod error;
pub mod middleware;
pub mod models;
pub mod openapi;
pub mod repository;
pub mod services;
pub mod state;
pub mod validation;
use axum::{Router, http};
use dotenv::dotenv;
use envy::from_env;
use http::Method;
use http::header::{AUTHORIZATION, CONTENT_TYPE};
use openapi::ApiDoc;
use state::AppState;
use tower_http::{
    LatencyUnit,
    cors::CorsLayer,
    trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer},
};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::config::Config;

pub fn build_router(state: AppState) -> Router {
    dotenv().ok();

    let config: Config = from_env().expect("Failed to load settings from env");

    let trace_layer = TraceLayer::new_for_http()
        .make_span_with(DefaultMakeSpan::new().include_headers(false))
        .on_response(
            DefaultOnResponse::new()
                .include_headers(false)
                .latency_unit(LatencyUnit::Micros),
        );

    let cors_layer = CorsLayer::new()
        .allow_origin(
            config
                .cors_origins
                .iter()
                .map(|o| o.parse().unwrap())
                .collect::<Vec<_>>(),
        )
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::PATCH,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([AUTHORIZATION, CONTENT_TYPE])
        .allow_credentials(true);

    let public_routes = Router::new()
        .nest("/api/health", api::health::routes())
        .nest("/api/auth", api::auth::routes());

    let protected_routes = Router::new()
        .nest("/api/users", api::user::routes())
        .nest("/api/jobs", api::job::routes())
        .layer(
            axum::middleware::from_fn_with_state(state.clone(), middleware::auth_middleware),
        );

    Router::new()
        .merge(SwaggerUi::new("/docs").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .merge(public_routes)
        .merge(protected_routes)
        .with_state(state)
        .layer(cors_layer)
        .layer(trace_layer)
}
