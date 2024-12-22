pub(crate) mod config;
pub(crate) mod database;
pub(crate) mod discord;
pub(crate) mod event;
pub(crate) mod storage;

use std::sync::Arc;

use anyhow::Context as _;
use config::GatewayConfig;
use event::{Context, RecognitionResults, RecognizedEventHandler};
use futures::StreamExt as _;
use tokio_util::task::TaskTracker;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let GatewayConfig {
        database_url,
        nats_url,
        discord_webhook_url,
        s3,
    } = config::parse_config()?;

    let nats_client = async_nats::connect(&nats_url)
        .await
        .context("Failed to connect to NATS")?;

    let storage = storage::Storage::from_config(s3).context("Failed to build storage")?;

    let context = Context {
        storage: Arc::new(storage),
    };

    let task_tracker = TaskTracker::new();

    let mut recognition_subscriber = nats_client.subscribe("recognition").await?;

    let publishers: Vec<Arc<dyn RecognizedEventHandler>> = vec![
        {
            let discord_handler = discord::DiscordHandler::new(&discord_webhook_url)?;
            Arc::new(discord_handler) as Arc<dyn RecognizedEventHandler>
        },
        {
            let database_handler = database::DatabaseHandler::connect(&database_url).await?;
            Arc::new(database_handler) as Arc<dyn RecognizedEventHandler>
        },
    ];

    while let Some(message) = recognition_subscriber.next().await {
        let recognition_result = match event::RecognitionResults::try_from(message) {
            Ok(result) => result,
            Err(err) => {
                tracing::error!("Failed to parse recognition result: {:?}", err);
                continue;
            }
        };

        tracing::debug!("Received recognition result: {recognition_result:?}");

        // if there is no result, skip the loop
        if recognition_result.results.is_empty() {
            continue;
        }

        // filter out the results that is not person
        let recognition_result = RecognitionResults {
            results: recognition_result
                .results
                .into_iter()
                .filter(|result| result.label == "person")
                .collect(),
        };

        let publishers = publishers.clone();

        for publisher in publishers {
            let recognition_result = recognition_result.clone();
            let context = context.clone();

            task_tracker.spawn(async move {
                publisher
                    .on_receive_recognition_result(&context, &recognition_result)
                    .await;
            });
        }
    }

    task_tracker.close();

    task_tracker.wait().await;

    Ok(())
}
