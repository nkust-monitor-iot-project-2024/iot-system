use std::time::Duration;

use crate::{prelude::*, storage::Storage};
use async_graphql::SimpleObject;
use sqlx::types::BigDecimal;

#[derive(SimpleObject)]
#[graphql(complex)]
pub struct Entity {
    pub id: i32,
    #[graphql(skip)]
    pub image_id: String,
    pub label: String,
    pub confidence: BigDecimal,
    pub created_at: time::PrimitiveDateTime,
}

static EXPIRE_AT: Duration = Duration::from_secs(3600);

#[async_graphql::ComplexObject]
impl Entity {
    /// Get the image of the entity.
    ///
    /// Note that it expires in 1 hour. Therefore, you should not save this URL.
    /// Always gets the image URL from the entity.
    ///
    /// If the URL expires, you might need to fetch the entity again to get a new URL.
    pub async fn image(
        &self,
        context: &async_graphql::Context<'_>,
    ) -> async_graphql::Result<String> {
        let storage = context.data::<Storage>()?;
        let path = format!("/{}", self.image_id);
        let image = storage.presign_read(&path, EXPIRE_AT).await?;

        Ok(image.uri().to_string())
    }
}
