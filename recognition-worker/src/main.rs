pub(crate) mod config;

use anyhow::Context;
use config::RecognitionConfig;
use futures::StreamExt as _;
use image::ImageFormat;
use ort::execution_providers::{CUDAExecutionProvider, CoreMLExecutionProvider};
use std::sync::Arc;
use tokio_util::task::TaskTracker;
use yolo_rs::model::YoloModelSession;
use yolo_rs::{image_to_yolo_input_tensor, inference};

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

    while let Some(frame_message) = frame_subscriber.next().await {
        let header_map = frame_message.headers.unwrap_or_default();
        let content_type = header_map.get("Content-Type").map(|ct| ct.to_string());

        if let Some(content_type) = content_type {
            if content_type != "image/png" {
                tracing::warn!(
                    "Receive a message with unsupported Content-Type: {}; skipping.",
                    content_type
                );
                continue;
            }
        } else {
            tracing::warn!("Receive a message without the Content-Type header; skipping.");
            continue;
        }

        let frame_id = header_map
            .get("Frame-Id")
            .map(|frame_id| frame_id.to_string());

        if let Some(ref frame_id) = frame_id {
            tracing::info!("Processing frame {}…", frame_id);
        } else {
            tracing::warn!(
                "Receive a message without the Frame-Id header. It is recommended to include the Frame-Id header."
            );
        }

        let yolo_model = yolo_model.clone();
        task_tracker.spawn_blocking(move || {
            tracing::info!("Processing frame in a blocking task…");

            let image = frame_message.payload;

            let image_reader = {
                let mut reader = image::ImageReader::new(std::io::Cursor::new(image));
                reader.set_format(ImageFormat::Png);

                reader
            };

            let image = match image_reader.decode() {
                Ok(frame_data) => frame_data,
                Err(e) => {
                    tracing::warn!("Failed to decode image: {:?}; skipping.", e);
                    return;
                }
            };

            let yolo_input = image_to_yolo_input_tensor(&image);
            let yolo_output =
                inference(&yolo_model, yolo_input.view()).expect("failed to run inference");

            tracing::info!("Found {} entities", yolo_output.len());

            for bounding_box in yolo_output {
                tracing::info!("Entity: {:?}", bounding_box);
            }
        });
    }

    Ok(())
}
