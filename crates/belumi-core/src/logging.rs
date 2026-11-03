use tracing_subscriber::EnvFilter;

use crate::config::BelumiConfig;
use crate::error::{BelumiError, BelumiResult};
use crate::paths::BelumiPaths;

pub fn init_logging(config: &BelumiConfig, _paths: &BelumiPaths) -> BelumiResult<()> {
    tracing_log::LogTracer::init().map_err(|e| BelumiError::LoggingInit(e.to_string()))?;

    let filter = build_filter(config);

    if config.logging.json {
        tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_target(true)
            .with_thread_ids(true)
            .json()
            .init();
    } else {
        tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_target(true)
            .with_thread_ids(true)
            .init();
    }

    tracing::info!(
        app = "Belumi",
        version = env!("CARGO_PKG_VERSION"),
        "logging initialized"
    );

    Ok(())
}

pub fn init_default() -> BelumiResult<()> {
    let config = BelumiConfig::default();
    let paths = BelumiPaths::new();
    let _ = paths.ensure_dirs();
    init_logging(&config, &paths)
}

fn build_filter(config: &BelumiConfig) -> EnvFilter {
    if let Ok(env_filter) = std::env::var("RUST_LOG") {
        if !env_filter.is_empty() {
            return EnvFilter::try_new(env_filter)
                .unwrap_or_else(|_| EnvFilter::new(config.logging.filter_string()));
        }
    }
    EnvFilter::new(config.logging.filter_string())
}

pub use tracing::{debug, error, info, trace, warn};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filter_string_valid() {
        let config = BelumiConfig::default();
        let filter = build_filter(&config);
        drop(filter);
    }
}
