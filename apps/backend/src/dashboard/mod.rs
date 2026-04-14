pub mod pages;
pub mod partials;

use axum::{Router, routing::get};
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(pages::dashboard))
        .route("/login", get(pages::login_page).post(pages::login_submit))
        .route("/partials/stats", get(partials::stats_fragment))
        .route("/partials/metrics", get(partials::metrics_fragment))
        .route("/partials/jobs", get(partials::jobs_fragment))
        .route("/partials/submit", get(partials::submit_form_fragment).post(partials::submit_job))
}
