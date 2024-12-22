use std::time::Duration;

use crate::{query::Monitor, storage::Storage};
use async_graphql::SimpleObject;

/// A detected entity.
#[derive(SimpleObject)]
#[graphql(complex)]
pub struct Entity {
    /// The ID of the entity.
    pub id: i32,
    #[graphql(skip)]
    pub image_id: String,
    /// The label of the entity.
    pub label: String,
    /// The confidence of the entity.
    ///
    /// It should be in the range of 0.0 to 1.0.
    pub confidence: bigdecimal::BigDecimal,
    /// The monitor of the entity.
    #[graphql(skip)]
    pub monitor_id: Option<String>,
    /// The time when the entity was detected.
    pub created_at: chrono::DateTime<chrono::Utc>,
}

static EXPIRE_AT: Duration = Duration::from_secs(3600);

#[async_graphql::ComplexObject]
impl Entity {
    /// Get the image URL of the entity.
    ///
    /// Note that it expires in 1 hour. Therefore, you should not save this URL.
    /// Always gets the image URL from the entity.
    ///
    /// If the URL expires, you might need to fetch the entity again to get a new URL.
    pub async fn url(&self, context: &async_graphql::Context<'_>) -> async_graphql::Result<String> {
        let storage = context.data::<Storage>()?;
        let path = format!("/{}", self.image_id);
        let image = storage.presign_read(&path, EXPIRE_AT).await?;

        Ok(image.uri().to_string())
    }

    pub async fn monitor(&self) -> Monitor {
        Monitor {
            id: self.monitor_id.clone(),
        }
    }
}
