use crate::prelude::*;
use crate::types::Entity;
use bigdecimal::BigDecimal;

pub struct MutationRoot;

#[Object]
impl MutationRoot {
    async fn create_entity(&self, context: &Context<'_>, image_id: String, label: String, confidence: BigDecimal) -> async_graphql::Result<Entity> {
        let pool = get_pgpool(context)?;

        let entity = sqlx::query_as!(Entity, "INSERT INTO entities (image_id, label, confidence) VALUES ($1, $2, $3) RETURNING id, image_id, label, confidence, created_at", image_id, label, confidence)
            .fetch_one(&pool)
            .await?;

        Ok(entity)
    }
}
