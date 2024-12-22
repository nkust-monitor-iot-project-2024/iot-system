use bigdecimal::FromPrimitive;

use crate::event::{Context, RecognitionResults, RecognizedEventHandler};

#[derive(Clone)]
pub struct DatabaseHandler {
    pool: sqlx::PgPool,
}

impl DatabaseHandler {
    pub async fn connect(url: &str) -> anyhow::Result<Self> {
        let pool = sqlx::PgPool::connect(url).await?;

        Ok(Self { pool })
    }
}

#[async_trait::async_trait]
impl RecognizedEventHandler for DatabaseHandler {
    #[tracing::instrument(skip(self, context))]
    async fn on_receive_recognition_result(&self, context: &Context, result: &RecognitionResults) {
        tracing::info!(
            "Received recognition result from the event bus and sending it to the database"
        );

        let storage = context.storage.clone();

        for result in &result.results {
            let image_key = match storage.put_recognition_result(&result).await {
                Ok(key) => key,
                Err(err) => {
                    tracing::error!("Failed to put recognition result to storage: {:?}", err);
                    continue;
                }
            };

            let confidence = bigdecimal::BigDecimal::from_f32(result.confidence)
                .map(|b| b.round(4))
                .unwrap_or_else(|| bigdecimal::BigDecimal::from_f32(0.0).unwrap());

            // check if there is such monitor_id in the database
            let monitor_id = sqlx::query!(
                r#"
                SELECT id FROM monitors WHERE id = $1
                "#,
                result.monitor_id
            )
            .fetch_one(&self.pool)
            .await;
            match monitor_id {
                Ok(_) => {}
                Err(sqlx::Error::RowNotFound) => {
                    sqlx::query!(
                        r#"
                        INSERT INTO monitors (id)
                        VALUES ($1)
                        "#,
                        result.monitor_id,
                    )
                    .execute(&self.pool)
                    .await
                    .unwrap();
                }
                Err(err) => {
                    tracing::error!("Failed to check if there is such monitor: {:?}", err);
                    continue;
                }
            }

            let _ = sqlx::query!(
                r#"
                INSERT INTO entities (image_id, monitor_id, confidence, label, created_at)
                VALUES ($1, $2, $3, $4, $5)
                "#,
                image_key,
                result.monitor_id,
                confidence,
                result.label,
                result.created_at,
            )
            .execute(&self.pool)
            .await
            .unwrap();
        }
    }
}
