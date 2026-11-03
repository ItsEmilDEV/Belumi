use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Default, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct BelumiConfig {
    #[serde(default)]
    pub server: ServerSection,
    #[serde(default)]
    pub tuning: Tuning,
}

#[derive(Debug, Default, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ServerSection {
    pub client: Option<PathBuf>,
    pub resources: Option<PathBuf>,
}

#[derive(Debug, Default, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Tuning {
    pub spawn_interval: Option<u32>,
    pub profile_seconds: Option<f32>,
}

impl BelumiConfig {
    pub fn parse(text: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(text)
    }

    pub fn env(&self) -> Vec<(&'static str, String)> {
        let mut v = Vec::new();
        if let Some(n) = self.tuning.spawn_interval.filter(|n| *n >= 1) {
            v.push(("BELUMI_SPAWN_INTERVAL", n.to_string()));
        }
        if let Some(s) = self.tuning.profile_seconds.filter(|s| *s > 0.0) {
            v.push(("BELUMI_PROFILE", s.to_string()));
        }
        v
    }
}

pub const TEMPLATE: &str = "\
# Belumi server settings. Plugins go in ./plugins/<name>/ (plugin.toml + plugin.wasm).

[server]
# client = \"/path/to/horizon-client\"      # built with: cargo build --profile fast -p horizon-client --no-default-features --features belumi
# resources = \"/path/to/.local/horizon-client/26.3\"

[tuning]
# Natural mob spawning every Nth tick; 1 is vanilla, 2-4 saves tick time on busy servers.
spawn_interval = 1
# Log a tick profile every N seconds (0 = off).
profile_seconds = 0
";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn template_parses_and_sets_nothing_by_default() {
        let c = BelumiConfig::parse(TEMPLATE).unwrap();
        assert_eq!(c.tuning.spawn_interval, Some(1));
        assert!(c.env().iter().all(|(k, _)| *k != "BELUMI_PROFILE"));
    }

    #[test]
    fn maps_to_env_and_rejects_typos() {
        let c =
            BelumiConfig::parse("[tuning]\nspawn_interval = 3\nprofile_seconds = 10.0\n").unwrap();
        assert_eq!(
            c.env(),
            vec![
                ("BELUMI_SPAWN_INTERVAL", "3".into()),
                ("BELUMI_PROFILE", "10".into())
            ]
        );
        assert!(BelumiConfig::parse("[tuning]\nspawn_intervall = 3\n").is_err());
        assert!(BelumiConfig::default().env().is_empty());
    }
}
