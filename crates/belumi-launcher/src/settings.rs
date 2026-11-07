use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const BUILT_IN_CLIENT_ID: &str = "142091da-91bb-4712-8a2c-06755a18080d";

const RETIRED_CLIENT_IDS: &[&str] = &[
    "594ada3c-f3ab-4d1e-a381-9a8d80b6dd72",
    "56e049ec-9827-4229-b7df-e6829beb50e4",
];

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
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub account: String,
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
    pub accounts: Vec<SavedAccount>,
    pub active_account: String,
    #[serde(rename = "account", skip_serializing_if = "Option::is_none")]
    legacy_account: Option<SavedAccount>,
    pub check_updates: bool,
    /// Discord Rich Presence of the launcher and, through `BELUMI_NO_DISCORD`, of the game.
    pub discord_presence: bool,
    /// The launcher's presence names the signed-in account.
    pub discord_show_account: bool,
    pub update_channel: String,
    pub style: String,
    pub language: String,
    pub game_dir: String,
    pub fullscreen: bool,
    pub window_width: u32,
    pub window_height: u32,
    pub graphics_api: String,
    pub after_start: String,
}

fn installed_client() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let client = exe.parent()?.join(format!("horizon-client{}", std::env::consts::EXE_SUFFIX));
    client.is_file().then_some(client)
}

fn installed_resources() -> Option<PathBuf> {
    installed_client()?;
    Some(belumi_core::paths::BelumiPaths::new().data_dir().join("resources/26.3"))
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
                version: String::new(),
                account: String::new(),
            }],
            selected_server: 0,
            client_binary: installed_client()
                .unwrap_or_else(|| horizon.join(format!("target/fast/horizon-client{}", std::env::consts::EXE_SUFFIX)))
                .display()
                .to_string(),
            resources: installed_resources()
                .unwrap_or_else(|| horizon.join(".local/horizon-client/26.3"))
                .display()
                .to_string(),
            offline_name: "Schnitzelblitz".into(),
            use_prism: false,
            prism_accounts_path: String::new(),
            accounts: Vec::new(),
            active_account: String::new(),
            legacy_account: None,
            check_updates: true,
            discord_presence: true,
            discord_show_account: false,
            update_channel: "stable".into(),
            style: "modern".into(),
            language: "auto".into(),
            game_dir: String::new(),
            fullscreen: false,
            window_width: 0,
            window_height: 0,
            graphics_api: "auto".into(),
            after_start: "keep".into(),
        }
    }
}

impl Settings {
    pub fn load(path: &Path) -> Self {
        let text = std::fs::read_to_string(path).ok();
        let parsed = text.as_deref().and_then(|t| serde_json::from_str(t).ok());
        if parsed.is_none() && text.as_ref().is_some_and(|t| !t.trim().is_empty()) {
            let _ = std::fs::copy(path, path.with_extension("json.bad"));
        }
        let mut settings: Self = parsed.unwrap_or_default();
        if settings.client_id.trim().is_empty() {
            settings.client_id = Self::default().client_id;
        } else if RETIRED_CLIENT_IDS.contains(&settings.client_id.trim()) {
            settings.client_id = Self::default().client_id;
            settings.accounts.clear();
            settings.legacy_account = None;
        }
        if let Some(old) = settings.legacy_account.take() {
            if !settings.accounts.iter().any(|a| a.uuid == old.uuid) {
                settings.accounts.insert(0, old);
            }
        }
        if !settings.accounts.iter().any(|a| a.uuid == settings.active_account) {
            settings.active_account = settings
                .accounts
                .first()
                .map_or_else(String::new, |a| a.uuid.clone());
        }
        if !settings.server.trim().is_empty() {
            let address = std::mem::take(&mut settings.server);
            if !settings.servers.iter().any(|s| s.address == address) {
                settings.servers.insert(
                    0,
                    ServerEntry {
                        name: address.clone(),
                        address,
                        version: String::new(),
                        account: String::new(),
                    },
                );
                settings.selected_server = 0;
            }
        }
        settings
    }

    pub fn active_saved(&self) -> Option<&SavedAccount> {
        self.accounts.iter().find(|a| a.uuid == self.active_account)
    }

    pub fn remember(&mut self, account: SavedAccount) {
        self.active_account = account.uuid.clone();
        match self.accounts.iter_mut().find(|a| a.uuid == account.uuid) {
            Some(existing) => *existing = account,
            None => self.accounts.push(account),
        }
    }

    pub fn forget(&mut self, uuid: &str) -> Option<SavedAccount> {
        let at = self.accounts.iter().position(|a| a.uuid == uuid)?;
        let removed = self.accounts.remove(at);
        if self.active_account == uuid {
            self.active_account = self
                .accounts
                .first()
                .map_or_else(String::new, |a| a.uuid.clone());
        }
        Some(removed)
    }

    pub fn active_version(&self) -> &str {
        self.servers
            .get(self.selected_server)
            .map_or("", |s| s.version.trim())
    }

    pub fn active_server(&self) -> &str {
        self.servers
            .get(self.selected_server)
            .map_or("", |s| s.address.trim())
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        use std::io::Write;
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let temp = path.with_extension("json.tmp");
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create(true).truncate(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options.open(&temp)?;
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
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sub/launcher.json");
        let mut settings = Settings::default();
        settings.servers.push(ServerEntry {
            name: "Ex".into(),
            address: "example.org:25570".into(),
            version: "26.2".into(),
            account: String::new(),
        });
        settings.remember(SavedAccount {
            name: "Alex".into(),
            uuid: "069a79f444e94726a5befca90e38aaf5".into(),
            refresh_token: "r".into(),
        });
        settings.save(&path).unwrap();
        assert_eq!(Settings::load(&path), settings);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
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
        assert_eq!(Settings::load(&bad).offline_name, "Schnitzelblitz");
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
    fn settings_from_before_versions_and_updates_still_load() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("old.json");
        std::fs::write(&path, r#"{"servers":[{"name":"A","address":"a:1"}],"selected_server":0}"#).unwrap();
        let loaded = Settings::load(&path);
        assert_eq!(loaded.active_server(), "a:1");
        assert_eq!(loaded.active_version(), "");
        assert!(loaded.check_updates);
        assert_eq!(loaded.update_channel, "stable");
    }

    #[test]
    fn the_single_saved_account_of_older_versions_becomes_the_list() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("old.json");
        std::fs::write(
            &path,
            r#"{"account":{"name":"Old","uuid":"u1","refresh_token":""}}"#,
        )
        .unwrap();
        let loaded = Settings::load(&path);
        assert_eq!(loaded.accounts.len(), 1);
        assert_eq!(loaded.active_saved().unwrap().name, "Old");
        assert!(!serde_json::to_string(&loaded).unwrap().contains("\"account\":{"));
    }

    #[test]
    fn accounts_are_added_switched_and_forgotten() {
        let acct = |n: &str, u: &str| SavedAccount {
            name: n.into(),
            uuid: u.into(),
            refresh_token: "r".into(),
        };
        let mut s = Settings::default();
        s.remember(acct("A", "1"));
        s.remember(acct("B", "2"));
        assert_eq!(s.active_saved().unwrap().name, "B");
        s.remember(acct("A2", "1"));
        assert_eq!(s.accounts.len(), 2);
        assert_eq!(s.active_saved().unwrap().name, "A2");
        assert!(s.forget("1").is_some());
        assert_eq!(s.active_saved().unwrap().name, "B");
        s.forget("2");
        assert!(s.active_saved().is_none());
        assert!(s.forget("2").is_none());
    }

    #[test]
    fn an_out_of_range_selection_means_no_server() {
        let mut settings = Settings::default();
        settings.selected_server = 9;
        assert_eq!(settings.active_server(), "");
    }

    #[test]
    fn a_retired_client_id_in_the_saved_settings_is_replaced() {
        let dir = std::env::temp_dir().join(format!("belumi-retired-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("launcher.json");
        std::fs::write(
            &path,
            r#"{"client_id":"594ada3c-f3ab-4d1e-a381-9a8d80b6dd72","account":{"name":"a","uuid":"b","refresh_token":"c"}}"#,
        )
        .unwrap();
        let loaded = Settings::load(&path);
        assert_eq!(loaded.client_id, Settings::default().client_id);
        assert!(loaded.accounts.is_empty());
        std::fs::remove_dir_all(&dir).ok();
    }
}
