pub(crate) mod config;
pub(crate) mod recognizer;

use anyhow::Context;
use async_nats::HeaderMap;
use config::RecognitionConfig;
use futures::StreamExt as _;
use recognizer::{RecognitionPayload, RecognitionWorker};
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
            #[cfg(feature = "cuda")]
            {
                ort::execution_providers::CUDAExecutionProvider::default().build()
            },
            #[cfg(feature = "coreml")]
            {
                ort::execution_providers::CoreMLExecutionProvider::default().build()
            },
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
        tracing::debug!("Received a frame message.");

        let worker = worker.clone(); // cheap clone
        let task_tracker_clone = task_tracker.clone();
        let nats_client = nats_client.clone();

        task_tracker.spawn(async move {
            let payload: RecognitionPayload = match frame_message.try_into() {
                Ok(payload) => payload,
                Err(e) => {
                    tracing::warn!("Failed to parse the message: {:?}; skipping.", e);
                    return;
                }
            };
            let frame_id = payload.frame_id.clone();

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
                    tracing::error!(
                        "Failed to create a thread to recognize the picture: {:?}. It should not happened :(",
                        e
                    );
                    return;
                }
            };

            tracing::info!("Publishing the results to NATS.");

            let mut header = HeaderMap::new();
            header.append("Content-Type", "application/json");
            header.append("X-Frame-Id", frame_id);

            // send the recognized results to recognition channel
            // each picture maps to a list of recognized entities
            let serialized_result = serde_json::to_string(&results);
            let serde_results = match serialized_result {
                Ok(serde_results) => serde_results,
                Err(e) => {
                    tracing::error!("Failed to serialize the results: {:?}. It should not happened :(", e);
                    return;
                }
            };

            let publish_result = nats_client.publish_with_headers("recognition", header, serde_results.into()).await;
            if let Err(e) = publish_result {
                tracing::warn!("Failed to publish the results: {:?}.", e);
            }
        });
    }

    task_tracker.close();
    task_tracker.wait().await;

    Ok(())
}
