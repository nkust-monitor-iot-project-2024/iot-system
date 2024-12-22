use anyhow::Context;

pub struct ExtractorConfig {
    pub rtsp_url: String,
    pub monitor_id: Option<String>,
    pub nats_url: String,
    pub frame_interval: Option<usize>,
}

pub fn parse_config() -> anyhow::Result<ExtractorConfig> {
    let rtsp_url = std::env::var("EXTRACTOR_RTSP_URL").context("EXTRACTOR_RTSP_URL environment variable is not set. RTSP stream is required to receive the frames.")?;
    let monitor_id = std::env::var("EXTRACTOR_MONITOR_ID").ok();
    let nats_url = std::env::var("EXTRACTOR_NATS_URL").context("EXTRACTOR_NATS_URL environment variable is not set. NATS is required for the extractor to send the extracted frame.")?;
    let frame_interval = std::env::var("EXTRACTOR_FRAME_INTERVAL")
        .ok()
        .map(|v| v.parse().unwrap());

    if let None = monitor_id {
        tracing::warn!(
            "Monitor ID not set. The extractor will omit the monitor ID in messages. It's recommended to set a monitor ID to help users identify the frame source."
        );
    }

    Ok(ExtractorConfig {
        rtsp_url,
        monitor_id,
        nats_url,
        frame_interval,
    })
}
