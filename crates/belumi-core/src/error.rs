use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum BelumiError {
    #[error("failed to read configuration file at {path}: {source}")]
    ConfigRead {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to write configuration file at {path}: {source}")]
    ConfigWrite {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to parse configuration: {0}")]
    ConfigParse(#[from] toml::de::Error),

    #[error("failed to serialize configuration: {0}")]
    ConfigSerialize(#[from] toml::ser::Error),

    #[error("invalid configuration value for '{field}': {reason}")]
    ConfigValidation { field: String, reason: String },

    #[error("failed to create directory at {path}: {source}")]
    DirectoryCreate {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to initialize logging: {0}")]
    LoggingInit(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

pub type BelumiResult<T> = Result<T, BelumiError>;
