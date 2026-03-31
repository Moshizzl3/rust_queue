use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Serialize, ToSchema)]
pub struct DataResponse<T: Serialize> {
    pub data: T,
}

impl<T: Serialize> DataResponse<T> {
    pub fn new(data: T) -> Self {
        Self { data }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ListResponse<T: Serialize> {
    pub data: Vec<T>,
}

impl<T: Serialize> ListResponse<T> {
    pub fn new(data: Vec<T>) -> Self {
        Self { data }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CreatedResponse {
    pub id: Uuid,
}

impl CreatedResponse {
    pub fn new(id: Uuid) -> Self {
        Self { id }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct EmptyResponse {
    pub success: bool,
}

impl EmptyResponse {
    pub fn new() -> Self {
        Self { success: true }
    }
}

impl Default for EmptyResponse {
    fn default() -> Self {
        Self::new()
    }
}
