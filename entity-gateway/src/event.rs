use async_nats::Message;
use bytes::Bytes;
use serde::Deserialize;

pub trait RecognizedEventHandler {
    async fn on_receive_recognition_result(&self, result: &RecognitionResults);
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecognitionResult {
    pub frame_id: String,
    pub monitor_id: Option<String>,
    pub label: String,
    pub confidence: f32,
    pub picture: Bytes,
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
