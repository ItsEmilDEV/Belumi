use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const BUILT_IN_CLIENT_ID: &str = "142091da-91bb-4712-8a2c-06755a18080d";

pub fn built_in_client_id() -> Option<&'static str> {
    (BUILT_IN_CLIENT_ID != "PASTE-YOUR-AZURE-CLIENT-ID-HERE").then_some(BUILT_IN_CLIENT_ID)
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SavedAccount {
    pub name: String,
    pub uuid: String,
    pub refresh_token: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ServerEntry {
    pub name: String,
    pub address: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub client_id: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub server: String,
    pub servers: Vec<ServerEntry>,
    pub selected_server: usize,
    pub client_binary: String,
    pub resources: String,
    pub offline_name: String,
    pub use_prism: bool,
    pub prism_accounts_path: String,
    pub account: Option<SavedAccount>,
}

impl Default for Settings {
    fn default() -> Self {
        let root = std::env::var("BELUMI_REPO")
            .map(PathBuf::from)
            .or_else(|_| std::env::current_dir())
            .unwrap_or_default();
        let horizon = root.join("vendor/horizon");
        Self {
            client_id: std::env::var("BELUMI_MSA_CLIENT_ID")
                .or_else(|_| std::env::var("HORIZON_MSA_CLIENT_ID"))
                .ok()
                .filter(|id| !id.trim().is_empty())
                .or_else(|| built_in_client_id().map(str::to_owned))
                .unwrap_or_default(),
            server: String::new(),
            servers: vec![ServerEntry {
                name: "Local server".into(),
                address: "127.0.0.1:25565".into(),
            }],
            selected_server: 0,
            client_binary: horizon
                .join("target/fast/horizon-client")
                .display()
                .to_string(),
            resources: horizon
                .join(".local/horizon-client/26.3")
                .display()
                .to_string(),
            offline_name: "BelumiPlayer".into(),
            use_prism: false,
            prism_accounts_path: String::new(),
            account: None,
        }
    }
}

impl Settings {
    pub fn load(path: &Path) -> Self {
        let mut settings: Self = std::fs::read_to_string(path)
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default();
        if settings.client_id.trim().is_empty() {
            settings.client_id = Self::default().client_id;
        }
        if !settings.server.trim().is_empty() {
            let address = std::mem::take(&mut settings.server);
            if !settings.servers.iter().any(|s| s.address == address) {
                settings.servers.insert(
                    0,
                    ServerEntry {
                        name: address.clone(),
                        address,
                    },
                );
                settings.selected_server = 0;
            }
        }
        settings
    }

    pub fn active_server(&self) -> &str {
        self.servers
            .get(self.selected_server)
            .map_or("", |s| s.address.trim())
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        use std::io::Write;
        use std::os::unix::fs::OpenOptionsExt;
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let temp = path.with_extension("json.tmp");
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(&temp)?;
        file.write_all(serde_json::to_string_pretty(self)?.as_bytes())?;
        file.sync_all()?;
        std::fs::rename(temp, path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_and_is_owner_only() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sub/launcher.json");
        let mut settings = Settings::default();
        settings.servers.push(ServerEntry {
            name: "Ex".into(),
            address: "example.org:25570".into(),
        });
        settings.account = Some(SavedAccount {
            name: "Alex".into(),
            uuid: "069a79f444e94726a5befca90e38aaf5".into(),
            refresh_token: "r".into(),
        });
        settings.save(&path).unwrap();
        assert_eq!(Settings::load(&path), settings);
        assert_eq!(
            std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }

    #[test]
    fn the_built_in_client_id_is_a_guid_and_counts_as_configured() {
        let parts: Vec<&str> = BUILT_IN_CLIENT_ID.split('-').collect();
        let lengths: Vec<usize> = parts.iter().map(|p| p.len()).collect();
        assert_eq!(
            lengths,
            [8, 4, 4, 4, 12],
            "not a GUID: {BUILT_IN_CLIENT_ID}"
        );
        assert!(
            BUILT_IN_CLIENT_ID
                .chars()
                .all(|c| c == '-' || c.is_ascii_hexdigit())
        );
        assert_eq!(built_in_client_id(), Some(BUILT_IN_CLIENT_ID));
    }

    #[test]
    fn missing_or_broken_files_give_defaults() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(
            Settings::load(&dir.path().join("none.json")).active_server(),
            "127.0.0.1:25565"
        );
        let bad = dir.path().join("bad.json");
        std::fs::write(&bad, "{ not json").unwrap();
        assert_eq!(Settings::load(&bad).offline_name, "BelumiPlayer");
        std::fs::write(&bad, r#"{"server":"s:1"}"#).unwrap();
        let loaded = Settings::load(&bad);
        assert_eq!(loaded.active_server(), "s:1");
        assert!(
            !serde_json::to_string(&loaded)
                .unwrap()
                .contains("\"server\"")
        );
    }

    #[test]
    fn an_out_of_range_selection_means_no_server() {
        let mut settings = Settings::default();
        settings.selected_server = 9;
        assert_eq!(settings.active_server(), "");
    }
}
