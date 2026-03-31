use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Health {
    #[schema(example = "OK")]
    pub status: String,

    #[schema(example = "2024-12-15T10:30:00Z")]
    pub time: String,

    #[schema(example = "1.0.0")]
    pub version: String,
}
