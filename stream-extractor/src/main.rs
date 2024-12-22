pub(crate) mod config;
pub(crate) mod worker;

use anyhow::Context;
use async_nats::HeaderMap;
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
        monitor_id,
        nats_url,
        frame_interval,
    } = config::parse_config()?;

    let (sender, receiver) = bounded::<(usize, DynamicImage)>(30); // 30s

    let nats_client = async_nats::connect(&nats_url)
        .await
        .context("Failed to connect to NATS")?;

    let task_tracker = TaskTracker::new();

    let extractor_worker_thread = task_tracker.spawn_blocking::<_, anyhow::Result<()>>(move || {
        let extractor_worker = worker::ExtractorWorkerBuilder {
            rtsp_url,
            sender,
            frame_interval,
        }
        .build()
        .context("Failed to build extractor worker")?;

        extractor_worker
            .set_state(gst::State::Playing)
            .context("failed to start extractor worker")?;

        // Wait until error or EOS ()
        let bus = extractor_worker.bus().context("failed to get bus")?;
        for msg in bus.iter_timed(gst::ClockTime::from_seconds(30)) {
            match msg.view() {
                gst::MessageView::Eos(..) => {
                    tracing::info!("End of stream. You should restart the worker manaully if there is a new stream.");
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
            .context("failed to stop extractor worker")?;

        Ok(())
    });

    for (frame_id, frame) in receiver {
        tracing::info!("Received frame {frame_id} from monitor {monitor_id:?}; sending to NATS");

        let nats_client = nats_client.clone();
        let monitor_id = monitor_id.clone();

        task_tracker.spawn(async move {
            let mut buf = Vec::new();
            let mut cursor = std::io::Cursor::new(&mut buf);

            if let Err(e) = frame.write_to(&mut cursor, ImageFormat::Png) {
                tracing::error!("Failed to write frame to PNG: {:?}", e);
                return;
            }

            let bytes = Bytes::from(buf);

            let mut nats_header = HeaderMap::new();
            nats_header.append("Content-Type", "image/png");
            nats_header.append("Date", chrono::Utc::now().to_rfc3339());
            nats_header.append("Frame-Id", frame_id.to_string());

            if let Some(monitor_id) = monitor_id {
                nats_header.append("Monitor-Id", monitor_id);
            }

            // publish the frame to NATS
            let result = nats_client
                .publish_with_headers("frames", nats_header, bytes)
                .await;
            if let Err(err) = result {
                tracing::error!("Failed to publish frame to NATS: {:?}", err);
                return;
            }
        });
    }

    task_tracker.close();

    if let Err(e) = extractor_worker_thread.await {
        tracing::error!("Extractor worker thread failed: {:?}", e);
    }

    task_tracker.wait().await;

    Ok(())
}
