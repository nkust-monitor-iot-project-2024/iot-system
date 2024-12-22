pub(crate) mod config;
pub(crate) mod event;

#[cfg(feature = "discord")]
pub(crate) mod discord;

use anyhow::Context;
use config::GatewayConfig;
use event::RecognizedEventHandler;
use futures::StreamExt as _;
use tokio_util::task::TaskTracker;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let GatewayConfig {
        nats_url,
        discord_webhook_url,
    } = config::parse_config()?;

    let nats_client = async_nats::connect(&nats_url)
        .await
        .context("Failed to connect to NATS")?;

    let task_tracker = TaskTracker::new();

    let mut recognition_subscriber = nats_client.subscribe("recognition").await?;

    let publishers = vec![
        #[cfg(feature = "discord")]
        {
            let discord_handler = discord::DiscordHandler::new(&discord_webhook_url)?;

            discord_handler
        },
    ];

    while let Some(message) = recognition_subscriber.next().await {
        tracing::debug!("Received an recognition message.");

        let recognition_result = match event::RecognitionResults::try_from(message) {
            Ok(result) => result,
            Err(err) => {
                tracing::error!("Failed to parse recognition result: {:?}", err);
                continue;
            }
        };

        // if there is no result, skip the loop
        if recognition_result.results.is_empty() {
            continue;
        }

        let publishers = publishers.clone();

        for publisher in publishers {
            let recognition_result = recognition_result.clone();

            task_tracker.spawn(async move {
                publisher
                    .on_receive_recognition_result(&recognition_result)
                    .await;
            });
        }
    }

    task_tracker.close();

    task_tracker.wait().await;

    Ok(())
}