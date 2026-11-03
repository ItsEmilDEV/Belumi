use crate::{ABI_VERSION, EventKind};
use serde::{Deserialize, Serialize};

#[derive(Debug, thiserror::Error)]
pub enum ManifestError {
    #[error("invalid plugin.toml: {0}")]
    Parse(#[from] toml::de::Error),
    #[error("plugin name {0:?} must be 1-32 chars of a-z, 0-9, '-' or '_'")]
    BadName(String),
    #[error("command name {0:?} must be 1-32 chars of a-z, 0-9, '-' or '_'")]
    BadCommand(String),
    #[error("plugin built for ABI {found}, server speaks ABI {expected}")]
    AbiMismatch { found: u32, expected: u32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Permission {
    Messaging,
    Kick,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginManifest {
    pub name: String,
    pub version: String,
    pub api_version: u32,
    #[serde(default = "default_main")]
    pub main: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub authors: Vec<String>,
    #[serde(default)]
    pub depends: Vec<String>,
    #[serde(default)]
    pub subscribe: Vec<EventKind>,
    #[serde(default)]
    pub permissions: Vec<Permission>,
    #[serde(default)]
    pub commands: Vec<String>,
}

fn default_main() -> String {
    "plugin.wasm".into()
}

fn valid_name(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 32
        && s.chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_')
}

impl PluginManifest {
    pub fn parse(text: &str) -> Result<Self, ManifestError> {
        let m: PluginManifest = toml::from_str(text)?;
        if !valid_name(&m.name) {
            return Err(ManifestError::BadName(m.name));
        }
        if let Some(bad) = m.commands.iter().find(|c| !valid_name(c)) {
            return Err(ManifestError::BadCommand(bad.clone()));
        }
        if m.api_version != ABI_VERSION {
            return Err(ManifestError::AbiMismatch {
                found: m.api_version,
                expected: ABI_VERSION,
            });
        }
        Ok(m)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_minimal() {
        let m = PluginManifest::parse("name='hello'\nversion='1.0.0'\napi_version=1\n").unwrap();
        assert_eq!(m.main, "plugin.wasm");
        assert!(m.subscribe.is_empty());
    }

    #[test]
    fn rejects_bad_name_and_abi() {
        assert!(matches!(
            PluginManifest::parse("name='Bad Name'\nversion='1'\napi_version=1\n"),
            Err(ManifestError::BadName(_))
        ));
        assert!(matches!(
            PluginManifest::parse("name='ok'\nversion='1'\napi_version=9\n"),
            Err(ManifestError::AbiMismatch { found: 9, .. })
        ));
    }

    #[test]
    fn rejects_bad_command() {
        assert!(matches!(
            PluginManifest::parse("name='ok'\nversion='1'\napi_version=1\ncommands=['Foo Bar']\n"),
            Err(ManifestError::BadCommand(_))
        ));
        let m =
            PluginManifest::parse("name='ok'\nversion='1'\napi_version=1\ncommands=['hello']\n")
                .unwrap();
        assert_eq!(m.commands, vec!["hello"]);
    }

    #[test]
    fn event_roundtrip_and_cancellable() {
        let e = crate::Event::Chat {
            uuid: "u".into(),
            message: "hi".into(),
        };
        let json = serde_json::to_string(&e).unwrap();
        assert!(json.contains("\"type\":\"chat\""));
        assert_eq!(serde_json::from_str::<crate::Event>(&json).unwrap(), e);
        assert!(e.cancellable());
        assert!(!crate::Event::Tick { tick: 1 }.cancellable());
    }
}
