use std::collections::HashMap;

use anyhow::Context;
use config::{Environment, File, FileFormat, builder::DefaultState};
use dotenvy::vars;

pub struct Config {
    pub database_url: String,
}

pub fn parse_config() -> anyhow::Result<Config> {
    let dotenv_variables = HashMap::from_iter(vars());

    let config = config::ConfigBuilder::<DefaultState>::default()
        .add_source(Environment::default().prefix("IOT"))
        .add_source(Environment::default().source(Some(dotenv_variables)))
        .add_source(File::new("config.toml", FileFormat::Toml).required(false))
        .build()
        .context("Failed to build configuration")?;

    let database_url = config
        .get_string("database_url")
        .context("You should define the DATABASE_URL.")?;

    let config = Config { database_url };

    Ok(config)
}
