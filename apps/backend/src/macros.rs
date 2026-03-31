#[macro_export]
macro_rules! impl_read_repository {
    (
        $repo:ty,
        $entity:ty,
        $table:literal,
        sort_fields = [$($sort_field:literal),*],
        filter_fields = [$($filter_field:literal),*]
    ) => {
        #[async_trait::async_trait]
        impl $crate::repository::ReadRepository<$entity> for $repo {
            async fn find_by_id(&self, id: uuid::Uuid) -> Result<Option<$entity>, sqlx::Error> {
                sqlx::query_as::<_, $entity>(concat!("SELECT * FROM ", $table, " WHERE id = $1"))
                    .bind(id)
                    .fetch_optional(self.pool())
                    .await
            }

            async fn find_all(
                &self,
                pagination: &$crate::models::pagination::PaginationParams,
            ) -> Result<$crate::models::pagination::PagedData<$entity>,  $crate::repository::FilterError> {
                self.find_filtered(&$crate::repository::FilterParams::new(), pagination).await
            }

            async fn find_filtered(
                &self,
                filters: &$crate::repository::FilterParams,
                pagination: &$crate::models::pagination::PaginationParams,
            ) -> Result<$crate::models::pagination::PagedData<$entity>,  $crate::repository::FilterError> {
                let allowed_sort_fields: &[&str] = &[$($sort_field),*];
                let allowed_filter_fields: &[&str] = &[$($filter_field),*];

                filters.validate(allowed_filter_fields)?;

                // Build WHERE clause
                let mut where_clauses: Vec<String> = vec![];
                let mut bind_values: Vec<&$crate::repository::FilterValue> = vec![];
                let mut bind_index = 1;

                for (field, value) in &filters.filters {
                    if allowed_filter_fields.contains(&field.as_str()) {
                        where_clauses.push(format!("{} = ${}", field, bind_index));
                        bind_values.push(value);
                        bind_index += 1;
                    }
                }

                let where_sql = if where_clauses.is_empty() {
                    String::new()
                } else {
                    format!("WHERE {}", where_clauses.join(" AND "))
                };

                // Validate sort field
                let sort_by = pagination.sort_by();
                let sort_field = if allowed_sort_fields.contains(&sort_by) {
                    sort_by
                } else {
                    "created_at"
                };

                // Count query
                let count_query = format!("SELECT COUNT(*) FROM {} {}", $table, where_sql);
                let total: (i64,) = $crate::repository::bind_filter_values(
                    sqlx::query_as(&count_query),
                    &bind_values,
                )
                .fetch_one(self.pool())
                .await?;

                // Data query
                let data_query = format!(
                    "SELECT * FROM {} {} ORDER BY {} {} LIMIT ${} OFFSET ${}",
                    $table,
                    where_sql,
                    sort_field,
                    pagination.sort_order().as_sql(),
                    bind_index,
                    bind_index + 1
                );

                let data = $crate::repository::bind_filter_values(
                    sqlx::query_as(&data_query),
                    &bind_values,
                )
                .bind(pagination.limit())
                .bind(pagination.offset())
                .fetch_all(self.pool())
                .await?;

                Ok($crate::models::pagination::PagedData {
                    data,
                    total: total.0,
                })
            }

            async fn find_one(
                &self,
                filters: &$crate::repository::FilterParams,
            ) -> Result<Option<$entity>,  $crate::repository::FilterError> {
                let allowed_filter_fields: &[&str] = &[$($filter_field),*];
                filters.validate(allowed_filter_fields)?;

                let mut where_clauses: Vec<String> = vec![];
                let mut bind_values: Vec<&$crate::repository::FilterValue> = vec![];
                let mut bind_index = 1;

                for (field, value) in &filters.filters {
                    if allowed_filter_fields.contains(&field.as_str()) {
                        where_clauses.push(format!("{} = ${}", field, bind_index));
                        bind_values.push(value);
                        bind_index += 1;
                    }
                }

                let where_sql = if where_clauses.is_empty() {
                    String::new()
                } else {
                    format!("WHERE {}", where_clauses.join(" AND "))
                };

                let query = format!("SELECT * FROM {} {} LIMIT 1", $table, where_sql);

                let result = $crate::repository::bind_filter_values(
                    sqlx::query_as(&query),
                    &bind_values,
                )
                .fetch_optional(self.pool())
                .await?;

                Ok(result)
            }

            async fn delete(&self, id: uuid::Uuid) -> Result<bool, sqlx::Error> {
                let result = sqlx::query(concat!("DELETE FROM ", $table, " WHERE id = $1"))
                    .bind(id)
                    .execute(self.pool())
                    .await?;
                Ok(result.rows_affected() > 0)
            }

            async fn exists(&self, id: uuid::Uuid) -> Result<bool, sqlx::Error> {
                let result: (bool,) = sqlx::query_as(concat!(
                    "SELECT EXISTS(SELECT 1 FROM ",
                    $table,
                    " WHERE id = $1)"
                ))
                .bind(id)
                .fetch_one(self.pool())
                .await?;
                Ok(result.0)
            }

            async fn count(&self) -> Result<i64, sqlx::Error> {
                let result: (i64,) = sqlx::query_as(concat!("SELECT COUNT(*) FROM ", $table))
                    .fetch_one(self.pool())
                    .await?;
                Ok(result.0)
            }
        }
    };
}
