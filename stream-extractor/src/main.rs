pub(crate) mod config;
pub(crate) mod worker;

use anyhow::Context;
use base64::prelude::*;
use bytes::Bytes;
use config::ExtractorConfig;
use crossbeam::channel::bounded;
use gst::prelude::*;
use gstreamer::prelude::ElementExt;
use gstreamer::{self as gst};
use image::{DynamicImage, ImageFormat};
use tokio_util::task::TaskTracker;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    // Initialize GStreamer
    gst::init()?;

    let ExtractorConfig {
        rtsp_url,
        nats_url,
        frame_interval,
    } = config::parse_config()?;

    let (sender, receiver) = bounded::<(usize, DynamicImage)>(30); // 30s

    let extractor_worker = worker::ExtractorWorkerBuilder {
        rtsp_url,
        sender,
        frame_interval,
    }
    .build()
    .context("Failed to build extractor worker")?;

    let nats_client = async_nats::connect(&nats_url)
        .await
        .context("Failed to connect to NATS")?;

    let task_tracker = TaskTracker::new();

    let _ = task_tracker.spawn_blocking(move || {
        extractor_worker
            .set_state(gst::State::Playing)
            .context("failed to start extractor worker")?;

        // Wait until error or EOS ()
        let bus = extractor_worker.bus().context("failed to get bus")?;
        for msg in bus.iter_timed(gst::ClockTime::from_seconds(30)) {
            match msg.view() {
                gst::MessageView::Eos(..) => {
                    tracing::info!("End of stream. Restart the worker if there is a new stream.");
                    break;
                }
                gst::MessageView::Error(err) => {
                    tracing::error!(
                        "Error from {}: {}",
                        err.src().map(|s| s.path_string()).unwrap_or("<?>".into()),
                        err.error()
                    );
                    break;
                }
                _ => (),
            }
        }

        extractor_worker
            .set_state(gst::State::Null)
            .context("failed to stop extractor worker")
    });

    for (frame_id, frame) in receiver {
        tracing::info!("Received frame {:}: send to NATS", frame_id);

        let nats_client = nats_client.clone();

        task_tracker.spawn(async move {
            let mut buf = Vec::new();
            let mut cursor = std::io::Cursor::new(&mut buf);

            if let Err(e) = frame.write_to(&mut cursor, ImageFormat::Png) {
                tracing::error!("Failed to write frame to PNG: {:?}", e);
                return;
            }

            // encode the PNG frame to Base64
            let base64_frame = BASE64_STANDARD.encode(&buf);

            let bytes = Bytes::from(base64_frame);

            // publish the frame to NATS
            let result = nats_client.publish("frames", bytes).await;
            if let Err(err) = result {
                tracing::error!("Failed to publish frame to NATS: {:?}", err);
                return;
            }
        });
    }

    task_tracker.wait().await;

    Ok(())
}
