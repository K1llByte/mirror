use std::{net::SocketAddr, path::Path, str::FromStr};

use serde::Deserialize;
use thiserror::Error;

#[derive(Deserialize)]
pub struct Config {
    #[serde(default = "default_host")]
    pub host: SocketAddr,
    pub bootstrap_peers: Vec<SocketAddr>,
}

fn default_host() -> SocketAddr {
    SocketAddr::from_str("0.0.0.0:2020").unwrap()
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("{0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Toml(#[from] toml::de::Error),
}

type ConfigResult<T> = Result<T, ConfigError>;

impl Config {
    pub async fn from_file<P: AsRef<Path>>(path: P) -> ConfigResult<Self> {
        let content = tokio::fs::read_to_string(path).await?;
        let config = toml::from_str(&content)?;
        Ok(config)
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            host: default_host(),
            bootstrap_peers: vec![],
        }
    }
}
