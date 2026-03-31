use dotenv::dotenv;
use envy::from_env;
use rust_queue::{build_router, config::Config, state::AppState};
use tracing::info;

#[tokio::main]
async fn main() {
    dotenv().ok();

    let config: Config = from_env().expect("Failed to load settings from env");

    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_ansi(!config.running_in_cloud)
        .init();

    let state = AppState::new(
        &config.database_url,
        &config.password_pepper,
        &config.jwt_secret,
        config.jwt_access_expiry_mins.unwrap_or(15),
        config.jwt_refresh_expiry_days.unwrap_or(15),
    )
    .await
    .expect("Failed to connect to database");

    let app = build_router(state);

    let bind_address = format!("0.0.0.0:{}", config.port);
    let listener = tokio::net::TcpListener::bind(bind_address).await.unwrap();

    info!("Listening on {}", listener.local_addr().unwrap());

    axum::serve(listener, app).await.unwrap();
}
