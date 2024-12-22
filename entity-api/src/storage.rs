use std::ops::Deref;

use anyhow::Context;
use opendal::{Configurator, Operator, layers::LoggingLayer, services::S3Config};

pub struct Storage {
    operator: Operator,
}

impl Storage {
    pub fn new(operator: Operator) -> Self {
        Self { operator }
    }

    pub fn from_config(config: S3Config) -> anyhow::Result<Self> {
        let client = config.into_builder();

        let operator = Operator::new(client)
            .context("Failed to build OpenDAL operator for S3")?
            .layer(LoggingLayer::default())
            .finish();

        Ok(Self::new(operator))
    }
}

impl Deref for Storage {
    type Target = Operator;

    fn deref(&self) -> &Self::Target {
        &self.operator
    }
}
