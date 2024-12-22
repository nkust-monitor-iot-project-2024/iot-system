pub(crate) mod config;
pub(crate) mod recognizer;

use anyhow::Context;
use config::RecognitionConfig;
use futures::StreamExt as _;
use ort::execution_providers::{CUDAExecutionProvider, CoreMLExecutionProvider};
use recognizer::RecognitionWorker;
use std::sync::Arc;
use tokio_util::task::TaskTracker;
use yolo_rs::model::YoloModelSession;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let RecognitionConfig { nats_url } = config::parse_config()?;

    // Initialize ONNX runtime
    ort::init()
        .with_execution_providers([
            CUDAExecutionProvider::default().build(),
            CoreMLExecutionProvider::default().build(),
        ])
        .with_telemetry(true)
        .commit()
        .context("failed to initialize ONNX runtime")?;

    let nats_client = async_nats::connect(&nats_url)
        .await
        .context("Failed to connect to NATS")?;

    let task_tracker = TaskTracker::new();

    let mut frame_subscriber = nats_client.subscribe("frames").await?;

    let yolo_model = Arc::new(
        YoloModelSession::from_filename_v8("models/yolo11x.onnx")
            .expect("failed to load YOLO model"),
    );
    let worker = RecognitionWorker::new(yolo_model);

    while let Some(frame_message) = frame_subscriber.next().await {
        let worker = worker.clone(); // cheap clone
        let task_tracker_clone = task_tracker.clone();

        task_tracker.spawn(async move {
            let payload = match frame_message.try_into() {
                Ok(payload) => payload,
                Err(e) => {
                    tracing::warn!("Failed to parse the message: {:?}; skipping.", e);
                    return;
                }
            };

            let results = task_tracker_clone
                .spawn_blocking(move || worker.recognize(payload))
                .await;

            let results = match results {
                Ok(Ok(results)) => results,
                Ok(Err(e)) => {
                    tracing::warn!("Failed to recognize the payload: {:?}; skipping.", e);
                    return;
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to create a thread to recognize the picture: {:?}; skipping.",
                        e
                    );
                    return;
                }
            };

            tracing::info!("Found {} entities", results.len());
            for result in results {
                tracing::info!("Entity: {:?}", result);
            }
        });
    }

    task_tracker.close();
    task_tracker.wait().await;

    Ok(())
}
