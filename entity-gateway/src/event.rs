use std::sync::Arc;

use async_nats::Message;
use bytes::Bytes;
use image::ImageFormat;
use serde::Deserialize;

use crate::storage::Storage;

/// The context of the event.
///
/// The [`Clone`] operation is cheap.
#[derive(Clone)]
pub struct Context {
    pub storage: Arc<Storage>,
}

#[async_trait::async_trait]
pub trait RecognizedEventHandler: Sync + Send {
    async fn on_receive_recognition_result(&self, context: &Context, result: &RecognitionResults);
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(unused)]
pub struct RecognitionResult {
    pub frame_id: String,
    pub monitor_id: Option<String>,
    pub label: String,
    pub confidence: f32,
    pub picture: Bytes,
    pub picture_type: ImageFormat,
    pub created_at: chrono::DateTime<chrono::FixedOffset>,
}

#[derive(Debug, Clone)]
pub struct RecognitionResults {
    pub results: Vec<RecognitionResult>,
}

impl TryFrom<Message> for RecognitionResults {
    type Error = anyhow::Error;

    fn try_from(message: Message) -> Result<Self, Self::Error> {
        let payload = message.payload;

        let payload = std::str::from_utf8(&payload)?;
        let results = serde_json::from_str(payload)?;

        Ok(RecognitionResults { results })
    }
}
