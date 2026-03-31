use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use super::base::BaseRepository;
use crate::models::user::{CreateUserRequest, UpdateUserRequest, User};
use crate::repository::WriteRepository;

#[derive(Clone)]
pub struct UserRepository {
    base: BaseRepository,
}

impl UserRepository {
    pub fn new(pool: PgPool) -> Self {
        Self {
            base: BaseRepository::new(pool),
        }
    }

    pub fn pool(&self) -> &PgPool {
        &self.base.pool
    }
}

crate::impl_read_repository!(
    UserRepository,
    User,
    "users",
    sort_fields = ["created_at", "updated_at", "email", "name", "role"],
    filter_fields = ["email", "name", "role"]
);

// write operations
#[async_trait]
impl WriteRepository<User, CreateUserRequest, UpdateUserRequest> for UserRepository {
    async fn create(&self, dto: CreateUserRequest) -> Result<User, sqlx::Error> {
        sqlx::query_as::<_, User>(
            r#"
            INSERT INTO users (id, email, name, role, password_hash, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, NOW(), NOW())
            RETURNING *
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(dto.email)
        .bind(dto.name)
        .bind(dto.role)
        .bind(dto.password)
        .fetch_one(self.pool())
        .await
    }

    async fn update(&self, id: Uuid, dto: UpdateUserRequest) -> Result<Option<User>, sqlx::Error> {
        sqlx::query_as::<_, User>(
            r#"
            UPDATE users 
            SET email = COALESCE($1, email),
                name = COALESCE($2, name),
                role = COALESCE($3, role),
                updated_at = NOW()
            WHERE id = $4
            RETURNING *
            "#,
        )
        .bind(dto.email)
        .bind(dto.name)
        .bind(dto.role)
        .bind(id)
        .fetch_optional(self.pool())
        .await
    }
}
