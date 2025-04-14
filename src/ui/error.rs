use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Failed parsing configuration yaml: {0}")]
    FailedToParse(#[from] serde_yaml::Error),
    #[error("Config failed to write to disk: {0}")]
    BadIO(#[from] std::io::Error),
}
