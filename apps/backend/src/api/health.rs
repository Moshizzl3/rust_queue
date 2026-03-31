use axum::{Json, Router, response::IntoResponse};
use chrono::Utc;

use crate::{
    models::health::{self, Health},
    state::AppState,
};

pub fn routes() -> Router<AppState> {
    Router::new().route("/", axum::routing::get(get_health_status))
}

#[utoipa::path(
    get,
    path = "/api/health",
    responses(
        (status = 200, description = "Returns health of the app.", body = health::Health)
    ),
    tag = "Health"
)]
async fn get_health_status() -> impl IntoResponse {
    Json(Health {
        status: "OK".to_string(),
        time: Utc::now().to_rfc3339(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}
