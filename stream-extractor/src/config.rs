use anyhow::Context;

pub struct ExtractorConfig {
    pub rtsp_url: String,
    pub nats_url: String,
    pub frame_interval: Option<usize>,
}

pub fn parse_config() -> anyhow::Result<ExtractorConfig> {
    let rtsp_url = std::env::var("EXTRACTOR_RTSP_URL").context("EXTRACTOR_RTSP_URL environment variable is not set. RTSP stream is required to receive the frames.")?;
    let nats_url = std::env::var("EXTRACTOR_NATS_URL").context("EXTRACTOR_NATS_URL environment variable is not set. NATS is required for the extractor to send the extracted frame.")?;
    let frame_interval = std::env::var("EXTRACTOR_FRAME_INTERVAL")
        .ok()
        .map(|v| v.parse().unwrap());

    Ok(ExtractorConfig {
        rtsp_url,
        nats_url,
        frame_interval,
    })
}
