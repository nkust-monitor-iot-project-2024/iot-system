use async_graphql::Context;

/// Get PostgreSQL connection pool from [`Context`].
pub fn get_pgpool(context: &Context<'_>) -> async_graphql::Result<sqlx::PgPool> {
    Ok(context.data::<sqlx::PgPool>()?.clone())
}
