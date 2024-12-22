use anyhow::Context;
use opendal::{Configurator, Operator, layers::LoggingLayer, services::S3Config};

use crate::event::RecognitionResult;

pub struct Storage {
    operator: Operator,
}

impl Storage {
    pub fn new(operator: Operator) -> Self {
        Self { operator }
    }

    pub fn from_config(config: S3Config) -> anyhow::Result<Self> {
        let client = config.into_builder();

        let operator = Operator::new(client)
            .context("Failed to build OpenDAL operator for S3")?
            .layer(LoggingLayer::default())
            .finish();

        Ok(Self::new(operator))
    }
}

impl Storage {
    /// Put the image in the recognition result to the storage.
    ///
    /// Returning the key of the image.
    pub async fn put_recognition_result(
        &self,
        result: &RecognitionResult,
    ) -> anyhow::Result<String> {
        let image_id = uuid::Uuid::new_v4();
        let image_key = format!("{}.{}", image_id, result.picture_type.extensions_str()[0]);

        self.operator
            .write(&image_key, result.picture.clone())
            .await?;

        Ok(image_key)
    }
}
