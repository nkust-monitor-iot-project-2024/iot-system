use std::{collections::BTreeMap, sync::Arc};

use crate::event::{Context, RecognitionResults, RecognizedEventHandler};

#[derive(Clone)]
pub struct DiscordHandler {
    client: Arc<discord_webhook2::webhook::DiscordWebhook>,
}

impl DiscordHandler {
    pub fn new(url: &str) -> anyhow::Result<Self> {
        let client = discord_webhook2::webhook::DiscordWebhook::new(url)?;

        Ok(Self {
            client: Arc::new(client),
        })
    }
}

#[async_trait::async_trait]
impl RecognizedEventHandler for DiscordHandler {
    #[tracing::instrument(skip(self))]
    async fn on_receive_recognition_result(&self, _: &Context, result: &RecognitionResults) {
        tracing::info!("Received recognition result from the event bus and sending it to Discord");

        for result in &result.results {
            let message = discord_webhook2::message::Message::new(|message| {
                message.embed(|embed| {
                    embed
                        .title("⚠️ 發現可疑物件 ⚠️")
                        .description("請到 App 中查看詳細資訊。")
                        .field(|field| field.name("發現時間").value(result.created_at.to_string()))
                        .field(|field| field.name("物件類型").value(&result.label))
                })
            });

            let mut files_entries = BTreeMap::new();
            files_entries.insert("picture.jpg".to_string(), result.picture.to_vec());

            let result = discord_webhook2::webhook::DiscordWebhook::send_with_files(
                &self.client,
                &message,
                files_entries,
            )
            .await;

            match result {
                Ok(id) => {
                    tracing::info!("Successfully sent the message to Discord: {id:?}");
                }
                Err(e) => {
                    tracing::error!("Failed to send the message to Discord: {e:?}");
                }
            }
        }
    }
}
