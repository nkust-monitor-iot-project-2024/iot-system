use anyhow::Context;

pub struct GatewayConfig {
    pub nats_url: String,
    pub discord_webhook_url: String,
}

pub fn parse_config() -> anyhow::Result<GatewayConfig> {
    let nats_url = std::env::var("GATEWAY_NATS_URL").context("GATEWAY_NATS_URL environment variable is not set. NATS is required for receiving the detected entities.")?;
    let discord_webhook_url = std::env::var("GATEWAY_DISCORD_WEBHOOK_URL").context("GATEWAY_DISCORD_WEBHOOK_URL environment variable is not set. Discord is required for sending the detected entities.")?;

    Ok(GatewayConfig {
        nats_url,
        discord_webhook_url,
    })
}
