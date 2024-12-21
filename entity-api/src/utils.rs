use async_graphql::{Context, SimpleObject};

/// Get PostgreSQL connection pool from [`Context`].
pub fn get_pgpool(context: &Context<'_>) -> async_graphql::Result<sqlx::PgPool> {
    Ok(context.data::<sqlx::PgPool>()?.clone())
}

#[derive(SimpleObject)]
pub struct PageInfo {
    pub has_next_page: bool,
    pub has_previous_page: bool,
    pub start_cursor: Option<i32>,
    pub end_cursor: Option<i32>,
}
