use std::path::PathBuf;

use crate::error::{BelumiError, BelumiResult};

pub const BELUMI_HOME_ENV: &str = "BELUMI_HOME";
pub const BELUMI_CONFIG_HOME_ENV: &str = "BELUMI_CONFIG_HOME";

#[derive(Debug, Clone)]
pub struct BelumiPaths {
    data_dir: PathBuf,
    config_dir: PathBuf,
}

impl BelumiPaths {
    pub fn new() -> Self {
        Self {
            data_dir: resolve_data_dir(),
            config_dir: resolve_config_dir(),
        }
    }

    pub fn with_dirs(data_dir: PathBuf, config_dir: PathBuf) -> Self {
        Self {
            data_dir,
            config_dir,
        }
    }

    pub fn data_dir(&self) -> &PathBuf {
        &self.data_dir
    }

    pub fn config_dir(&self) -> &PathBuf {
        &self.config_dir
    }

    pub fn config_file(&self) -> PathBuf {
        self.config_dir.join("belumi.toml")
    }

    pub fn logs_dir(&self) -> PathBuf {
        self.data_dir.join("logs")
    }

    pub fn ensure_dirs(&self) -> BelumiResult<()> {
        for dir in [&self.data_dir, &self.config_dir, &self.logs_dir()] {
            if !dir.exists() {
                std::fs::create_dir_all(dir).map_err(|source| BelumiError::DirectoryCreate {
                    path: dir.clone(),
                    source,
                })?;
            }
        }
        Ok(())
    }
}

impl Default for BelumiPaths {
    fn default() -> Self {
        Self::new()
    }
}

fn resolve_data_dir() -> PathBuf {
    if let Some(home) = env_non_empty(BELUMI_HOME_ENV) {
        return PathBuf::from(home);
    }
    if let Some(home) = env_non_empty("HORIZON_HOME") {
        return PathBuf::from(home);
    }
    platform_data_dir().join("belumi")
}

fn resolve_config_dir() -> PathBuf {
    if let Some(dir) = env_non_empty(BELUMI_CONFIG_HOME_ENV) {
        return PathBuf::from(dir);
    }
    if cfg!(windows) {
        if let Some(appdata) = env_non_empty("APPDATA") {
            return PathBuf::from(appdata).join("Belumi");
        }
    } else if cfg!(target_os = "macos") {
        if let Some(home) = env_non_empty("HOME") {
            return PathBuf::from(home)
                .join("Library")
                .join("Application Support")
                .join("Belumi");
        }
    } else {
        if let Some(xdg) = env_non_empty("XDG_CONFIG_HOME") {
            return PathBuf::from(xdg).join("belumi");
        }
        if let Some(home) = env_non_empty("HOME") {
            return PathBuf::from(home).join(".config").join("belumi");
        }
    }
    platform_data_dir().join("belumi").join("config")
}

fn platform_data_dir() -> PathBuf {
    if cfg!(windows) {
        if let Some(appdata) = env_non_empty("APPDATA") {
            return PathBuf::from(appdata);
        }
    } else if cfg!(target_os = "macos") {
        if let Some(home) = env_non_empty("HOME") {
            return PathBuf::from(home)
                .join("Library")
                .join("Application Support");
        }
    } else {
        if let Some(xdg) = env_non_empty("XDG_DATA_HOME") {
            return PathBuf::from(xdg);
        }
        if let Some(home) = env_non_empty("HOME") {
            return PathBuf::from(home).join(".local").join("share");
        }
    }
    std::env::temp_dir()
}

fn env_non_empty(key: &str) -> Option<String> {
    std::env::var(key).ok().filter(|v| !v.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn paths_are_not_empty() {
        let paths = BelumiPaths::new();
        assert!(!paths.data_dir().as_os_str().is_empty());
        assert!(!paths.config_dir().as_os_str().is_empty());
    }

    #[test]
    fn config_file_is_inside_config_dir() {
        let paths = BelumiPaths::with_dirs(PathBuf::from("/data"), PathBuf::from("/config"));
        assert_eq!(paths.config_file(), PathBuf::from("/config/belumi.toml"));
    }

    #[test]
    fn env_override_data_dir() {
        let paths = BelumiPaths::with_dirs(
            PathBuf::from("/tmp/test-data"),
            PathBuf::from("/tmp/test-config"),
        );
        assert_eq!(paths.data_dir(), Path::new("/tmp/test-data"));
    }

    #[test]
    fn ensure_dirs_creates_directories() {
        let tmp = std::env::temp_dir().join("belumi-paths-test");
        let _ = std::fs::remove_dir_all(&tmp);
        let paths = BelumiPaths::with_dirs(tmp.join("data"), tmp.join("config"));
        paths.ensure_dirs().unwrap();
        assert!(tmp.join("data").exists());
        assert!(tmp.join("config").exists());
        assert!(tmp.join("data").join("logs").exists());
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
