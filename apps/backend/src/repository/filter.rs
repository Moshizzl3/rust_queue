use sqlx::{Postgres, postgres::PgArguments, query::QueryAs};
use std::collections::HashMap;
use thiserror::Error;

/// Parameters for filtering database queries.
///
/// # Example
///
/// ```
/// use rust_queue::repository::FilterParams;
/// use uuid::Uuid;
///
/// let filters = FilterParams::new()
///     .add_string("city", "Copenhagen")
///     .add_uuid("owner_id", Uuid::new_v4());
/// ``
#[derive(Debug, Default, Clone)]
pub struct FilterParams {
    pub filters: HashMap<String, FilterValue>,
}

#[derive(Debug, Clone)]
pub enum FilterValue {
    String(String),
    Uuid(uuid::Uuid),
    Int(i64),
    Bool(bool),
}

#[derive(Debug, Error)]
pub enum FilterError {
    #[error("Invalid filter field: {0}. Allowed fields: {1}")]
    InvalidField(String, String),

    #[error("{0}")]
    Database(#[from] sqlx::Error),
}

impl FilterParams {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_string(mut self, field: &str, value: impl Into<String>) -> Self {
        self.filters
            .insert(field.to_string(), FilterValue::String(value.into()));
        self
    }

    pub fn add_uuid(mut self, field: &str, value: uuid::Uuid) -> Self {
        self.filters
            .insert(field.to_string(), FilterValue::Uuid(value));
        self
    }

    pub fn add_int(mut self, field: &str, value: i64) -> Self {
        self.filters
            .insert(field.to_string(), FilterValue::Int(value));
        self
    }

    pub fn add_bool(mut self, field: &str, value: bool) -> Self {
        self.filters
            .insert(field.to_string(), FilterValue::Bool(value));
        self
    }

    pub fn is_empty(&self) -> bool {
        self.filters.is_empty()
    }

    pub fn validate(&self, allowed_fields: &[&str]) -> Result<(), FilterError> {
        for field in self.filters.keys() {
            if !allowed_fields.contains(&field.as_str()) {
                return Err(FilterError::InvalidField(
                    field.clone(),
                    allowed_fields.join(", "),
                ));
            }
        }
        Ok(())
    }
}

/// Helper to bind filter values to a query
pub fn bind_filter_values<'q, T>(
    mut query: QueryAs<'q, Postgres, T, PgArguments>,
    values: &'q [&'q FilterValue],
) -> QueryAs<'q, Postgres, T, PgArguments>
where
    T: for<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow> + Send + Unpin,
{
    for value in values {
        query = match value {
            FilterValue::String(v) => query.bind(v),
            FilterValue::Uuid(v) => query.bind(v),
            FilterValue::Int(v) => query.bind(v),
            FilterValue::Bool(v) => query.bind(v),
        };
    }
    query
}
