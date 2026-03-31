use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use super::filter::FilterParams;
use crate::models::pagination::{PagedData, PaginationParams};
use crate::repository::filter::FilterError;

#[async_trait]
pub trait ReadRepository<T>: Send + Sync + Clone
where
    T: Send + Unpin,
{
    async fn find_by_id(&self, id: Uuid) -> Result<Option<T>, sqlx::Error>;
    async fn find_all(&self, pagination: &PaginationParams) -> Result<PagedData<T>, FilterError>;
    async fn find_filtered(
        &self,
        filters: &FilterParams,
        pagination: &PaginationParams,
    ) -> Result<PagedData<T>, FilterError>;
    async fn find_one(&self, filters: &FilterParams) -> Result<Option<T>, FilterError>;
    async fn delete(&self, id: Uuid) -> Result<bool, sqlx::Error>;
    async fn exists(&self, id: Uuid) -> Result<bool, sqlx::Error>;
    async fn count(&self) -> Result<i64, sqlx::Error>;
}
#[async_trait]
pub trait WriteRepository<T, CreateDTO, UpdateDTO>: Send + Sync {
    async fn create(&self, dto: CreateDTO) -> Result<T, sqlx::Error>;
    async fn update(&self, id: Uuid, dto: UpdateDTO) -> Result<Option<T>, sqlx::Error>;
}

#[derive(Clone)]
pub struct BaseRepository {
    pub pool: PgPool,
}

impl BaseRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}
