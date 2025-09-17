use std::path::Path;

use serde::Deserialize;
use thiserror::Error;

#[derive(Deserialize)]
pub struct Config {
    #[serde(default = "default_host")]
    pub host: String,
    pub bootstrap_peers: Vec<String>,
}

fn default_host() -> String {
    "0.0.0.0:2020".into()
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
            host: "127.0.0.1:2020".into(),
            bootstrap_peers: vec![],
        }
    }
}
