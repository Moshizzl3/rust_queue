use crate::{
    repository::UserRepository,
    services::{JwtService, PasswordService},
};
use sqlx::PgPool;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub users: UserRepository,
    pub password_service: PasswordService,
    pub jwt_service: JwtService,
}

impl AppState {
    pub async fn new(
        database_url: &str,
        password_pepper: &str,
        jwt_secret: &str,
        jwt_expiration_mins: i64,
        refresh_expiry_days: i64,
    ) -> Result<Self, sqlx::Error> {
        let pool = PgPool::connect(database_url).await?;
        sqlx::migrate!().run(&pool).await?;

        Ok(Self {
            pool: pool.clone(),
            users: UserRepository::new(pool.clone()),
            password_service: PasswordService::new(password_pepper.to_string()),
            jwt_service: JwtService::new(jwt_secret, jwt_expiration_mins, refresh_expiry_days),
        })
    }
}
