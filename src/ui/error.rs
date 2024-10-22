#[derive(Debug)]
pub enum ConfigError {
    FailedToParse(serde_yaml::Error),
    BadIO(std::io::Error),
}

impl From<serde_yaml::Error> for ConfigError {
    fn from(value: serde_yaml::Error) -> Self {
        ConfigError::FailedToParse(value)
    }
}

impl From<std::io::Error> for ConfigError {
    fn from(value: std::io::Error) -> Self {
        ConfigError::BadIO(value)
    }
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BadIO(e) => write!(f, "Config failed IO: {e}"),
            Self::FailedToParse(e) => write!(f, "Config failed to parse: {e}"),
        }
    }
}

impl std::error::Error for ConfigError {}
