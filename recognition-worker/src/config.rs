use anyhow::Context;

pub struct RecognitionConfig {
    pub nats_url: String,
}

pub fn parse_config() -> anyhow::Result<RecognitionConfig> {
    let nats_url = std::env::var("RECOGNITION_NATS_URL").context("RECOGNITION_NATS_URL environment variable is not set. NATS is required for the recognition worker to receive the sampled frame and send the recognized images.")?;

    Ok(RecognitionConfig { nats_url })
}
