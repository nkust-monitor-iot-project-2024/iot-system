use crate::prelude::*;
use crate::types::Entity;

pub struct QueryRoot;

#[Object]
impl QueryRoot {
    async fn entities(&self, context: &Context<'_>, cursor: Option<i32>, page: Option<i64>) -> async_graphql::Result<Vec<Entity>> {
        let pool = get_pgpool(context)?;

        let entites = sqlx::query_as!(Entity, "SELECT id, image_id, label, confidence, created_at FROM entities WHERE id < $1 ORDER BY id DESC LIMIT $2", cursor.unwrap_or(i32::MAX), page.unwrap_or(10))
            .fetch_all(&pool)
            .await?;

        Ok(entites)
    }

    async fn entity(&self, context: &Context<'_>, id: i32) -> async_graphql::Result<Entity> {
        let pool = get_pgpool(context)?;

        let entity = sqlx::query_as!(Entity, "SELECT id, image_id, label, confidence, created_at FROM entities WHERE id = $1", id)
            .fetch_one(&pool)
            .await?;

        Ok(entity)
    }
}
