use crate::prelude::*;
use bigdecimal::BigDecimal;

pub struct MutationRoot;

#[Object]
impl MutationRoot {
    /// Create an entity.
    ///
    /// It is for testing purposes only, and should be
    /// removed in production.
    ///
    /// # Arguments
    ///
    /// * `image_url` - The URL of the image. Accepting only the JPG.
    #[cfg(feature = "create-entity")]
    async fn create_entity(
        &self,
        context: &Context<'_>,
        image_url: String,
        label: String,
        confidence: BigDecimal,
    ) -> async_graphql::Result<i32> {
        use uuid::Uuid;

        // store image to S3
        let image_id = {
            let image_resp = reqwest::get(&image_url).await?;
            let image_bytes = image_resp.bytes().await?;

            let storage = context.data::<Storage>()?;
            let image_id = format!("{}.jpg", Uuid::new_v4().to_string());

            let path = format!("/{image_id}");
            storage.write(&path, image_bytes).await?;

            image_id
        };

        // store image to database
        let pool = context.data::<DatabasePool>()?.get_pool();

        let entity = sqlx::query!(
            "INSERT INTO entities (image_id, label, confidence) VALUES ($1, $2, $3) RETURNING id",
            image_id,
            label,
            confidence
        )
        .fetch_one(&pool)
        .await?;

        Ok(entity.id)
    }
}
