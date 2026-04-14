use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

#[derive(Debug, Clone, Deserialize, IntoParams, ToSchema)]
pub struct PaginationParams {
    #[param(default = 20, minimum = 1, maximum = 100)]
    pub limit: Option<i64>,
    #[param(default = 0, minimum = 0)]
    pub offset: Option<i64>,
    #[param(default = "created_at")]
    pub sort_by: Option<String>,
    #[param(default = "desc")]
    pub sort_order: Option<SortOrder>,
}

impl PaginationParams {
    pub fn new(limit: Option<i64>, offset: Option<i64>) -> Self {
        Self {
            limit,
            offset,
            sort_by: Some("created_at".to_string()),
            sort_order: Some(SortOrder::Desc),
        }
    }
}

impl Default for PaginationParams {
    fn default() -> Self {
        Self {
            limit: Some(20),
            offset: Some(0),
            sort_by: Some("created_at".to_string()),
            sort_order: Some(SortOrder::Desc),
        }
    }
}

impl PaginationParams {
    pub fn limit(&self) -> i64 {
        self.limit.unwrap_or(20).clamp(1, 100)
    }

    pub fn offset(&self) -> i64 {
        self.offset.unwrap_or(0).max(0)
    }

    pub fn sort_by(&self) -> &str {
        self.sort_by.as_deref().unwrap_or("created_at")
    }

    pub fn sort_order(&self) -> &SortOrder {
        self.sort_order.as_ref().unwrap_or(&SortOrder::Desc)
    }
}

#[derive(Debug, Clone, Deserialize, Default, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum SortOrder {
    Asc,
    #[default]
    Desc,
}

impl SortOrder {
    pub fn as_sql(&self) -> &'static str {
        match self {
            SortOrder::Asc => "ASC",
            SortOrder::Desc => "DESC",
        }
    }
}

#[derive(Debug, Clone)]
pub struct PagedData<T> {
    pub data: Vec<T>,
    pub total: i64,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PaginatedResponse<T: Serialize> {
    pub data: Vec<T>,
    pub pagination: PaginationMetadata,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PaginationMetadata {
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
    pub has_more: bool,
}

impl<T: Serialize> PaginatedResponse<T> {
    pub fn new(data: Vec<T>, total: i64, pagination: &PaginationParams) -> Self {
        let limit = pagination.limit();
        let offset = pagination.offset();

        Self {
            data,
            pagination: PaginationMetadata {
                total,
                limit,
                offset,
                has_more: offset + limit < total,
            },
        }
    }
}
