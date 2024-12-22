use std::collections::HashMap;

use anyhow::Context;
use config::{builder::DefaultState, Environment, File, FileFormat};
use dotenvy::vars;
use opendal::services::S3Config;

#[derive(serde::Deserialize)]
pub struct GatewayConfig {
    pub database_url: String,
    pub nats_url: String,
    pub discord_webhook_url: String,
    pub s3: S3Config,
}

pub fn parse_config() -> anyhow::Result<GatewayConfig> {
    let dotenv_variables = HashMap::from_iter(vars());

    let config = config::ConfigBuilder::<DefaultState>::default()
        .add_source(
            Environment::default()
                .prefix("IOT")
                .prefix_separator("_")
                .keep_prefix(false)
                .separator("__"),
        )
        .add_source(
            Environment::default()
                .source(Some(dotenv_variables))
                .separator("__"),
        )
        .add_source(File::new("config.toml", FileFormat::Toml).required(false))
        .build()
        .context("Failed to build configuration")?;

    let deserialized_config: GatewayConfig = config
        .try_deserialize()
        .context("Failed to deserialize configuration")?;

    Ok(deserialized_config)
}

