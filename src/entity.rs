use async_graphql::SimpleObject;
use sqlx::types::BigDecimal;

#[derive(SimpleObject)]
pub struct Entity {
    pub id: i32,
    pub image_id: String,
    pub label: String,
    pub confidence: BigDecimal,
    pub created_at: time::PrimitiveDateTime,
}
