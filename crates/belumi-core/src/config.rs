use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::error::{BelumiError, BelumiResult};
use crate::paths::BelumiPaths;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(default)]
pub struct BelumiConfig {
    pub general: GeneralConfig,
    pub logging: LoggingConfig,
}

impl BelumiConfig {
    pub fn load_or_default(path: &Path) -> BelumiResult<Self> {
        if !path.exists() {
            let config = Self::default();
            config.save(path)?;
            return Ok(config);
        }
        let content = std::fs::read_to_string(path).map_err(|source| BelumiError::ConfigRead {
            path: path.to_path_buf(),
            source,
        })?;
        let config: Self = toml::from_str(&content)?;
        config.validate()?;
        Ok(config)
    }

    pub fn load(path: &Path) -> BelumiResult<Self> {
        let content = std::fs::read_to_string(path).map_err(|source| BelumiError::ConfigRead {
            path: path.to_path_buf(),
            source,
        })?;
        let config: Self = toml::from_str(&content)?;
        config.validate()?;
        Ok(config)
    }

    pub fn save(&self, path: &Path) -> BelumiResult<()> {
        self.validate()?;
        let content = toml::to_string_pretty(self).map_err(BelumiError::ConfigSerialize)?;
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent).map_err(|source| BelumiError::DirectoryCreate {
                    path: parent.to_path_buf(),
                    source,
                })?;
            }
        }
        std::fs::write(path, content).map_err(|source| BelumiError::ConfigWrite {
            path: path.to_path_buf(),
            source,
        })?;
        Ok(())
    }

    pub fn validate(&self) -> BelumiResult<()> {
        self.general.validate()?;
        self.logging.validate()?;
        Ok(())
    }

    pub fn load_default() -> BelumiResult<Self> {
        let paths = BelumiPaths::new();
        paths.ensure_dirs()?;
        Self::load_or_default(&paths.config_file())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct GeneralConfig {
    pub app_name: String,
    pub debug: bool,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            app_name: "Belumi".to_string(),
            debug: false,
        }
    }
}

impl GeneralConfig {
    fn validate(&self) -> BelumiResult<()> {
        if self.app_name.is_empty() {
            return Err(BelumiError::ConfigValidation {
                field: "general.app_name".to_string(),
                reason: "must not be empty".to_string(),
            });
        }
        if self.app_name.len() > 128 {
            return Err(BelumiError::ConfigValidation {
                field: "general.app_name".to_string(),
                reason: "must be at most 128 characters".to_string(),
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct LoggingConfig {
    pub level: String,
    pub json: bool,
    pub file: bool,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            json: false,
            file: true,
        }
    }
}

impl LoggingConfig {
    fn validate(&self) -> BelumiResult<()> {
        let valid_levels = ["trace", "debug", "info", "warn", "error"];
        if !valid_levels.contains(&self.level.as_str()) {
            return Err(BelumiError::ConfigValidation {
                field: "logging.level".to_string(),
                reason: format!("must be one of: {}", valid_levels.join(", ")),
            });
        }
        Ok(())
    }

    pub fn filter_string(&self) -> String {
        format!("belumi_core={level},belumi={level}", level = self.level)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn default_config_is_valid() {
        let config = BelumiConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn default_config_roundtrip() {
        let config = BelumiConfig::default();
        let toml_str = toml::to_string_pretty(&config).unwrap();
        let parsed: BelumiConfig = toml::from_str(&toml_str).unwrap();
        assert_eq!(config, parsed);
    }

    #[test]
    fn config_save_and_load() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("belumi.toml");
        let config = BelumiConfig::default();
        config.save(&path).unwrap();
        assert!(path.exists());

        let loaded = BelumiConfig::load(&path).unwrap();
        assert_eq!(config, loaded);
    }

    #[test]
    fn load_or_default_creates_file() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("belumi.toml");
        assert!(!path.exists());
        let config = BelumiConfig::load_or_default(&path).unwrap();
        assert!(path.exists());
        assert_eq!(config, BelumiConfig::default());
    }

    #[test]
    fn invalid_log_level_rejected() {
        let mut config = BelumiConfig::default();
        config.logging.level = "invalid".to_string();
        assert!(config.validate().is_err());
    }

    #[test]
    fn empty_app_name_rejected() {
        let mut config = BelumiConfig::default();
        config.general.app_name = String::new();
        assert!(config.validate().is_err());
    }

    #[test]
    fn parsed_invalid_config_rejected() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("belumi.toml");
        let mut file = std::fs::File::create(&path).unwrap();
        writeln!(file, "general.app_name = \"\"").unwrap();
        assert!(BelumiConfig::load(&path).is_err());
    }

    #[test]
    fn filter_string_contains_level() {
        let config = BelumiConfig::default();
        let filter = config.logging.filter_string();
        assert!(filter.contains("info"));
    }
}
