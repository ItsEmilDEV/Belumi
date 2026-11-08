//! Switching accounts from inside the game. The launcher tells the game who is saved (names and ids only, never a
//! token) in `belumi-accounts.json` in the game folder. When the player picks another one the game writes
//! `belumi-switch.json` and quits; the launcher sees the game end, signs that account in and starts the game again on
//! the same server.
use crate::settings::Settings;
use std::path::Path;

pub const ACCOUNTS_FILE: &str = "belumi-accounts.json";
pub const REQUEST_FILE: &str = "belumi-switch.json";

/// What the game may know: the saved accounts' names and ids, which one is active, and whether anyone will start the
/// game again (not when the launcher closes itself at start).
pub fn accounts_json(settings: &Settings) -> String {
    let accounts: Vec<_> = settings
        .accounts
        .iter()
        .map(|a| serde_json::json!({"name": a.name, "uuid": a.uuid}))
        .collect();
    serde_json::json!({
        "active": settings.active_account,
        "can_switch": settings.after_start != "close",
        "accounts": accounts,
    })
    .to_string()
}

pub fn write_accounts(settings: &Settings, dir: &Path) {
    let _ = std::fs::write(dir.join(ACCOUNTS_FILE), accounts_json(settings));
}

/// The account and server of a switch request: `{"uuid": "...", "address": "host:port"}`; the address may be empty.
pub fn parse_request(text: &str) -> Option<(String, String)> {
    let value: serde_json::Value = serde_json::from_str(text).ok()?;
    let uuid = value.get("uuid")?.as_str()?.trim().to_owned();
    let address = value.get("address").and_then(|a| a.as_str()).unwrap_or("").trim().to_owned();
    (!uuid.is_empty() && address.len() <= 255 && !address.contains(char::is_whitespace)).then_some((uuid, address))
}

/// Reads and deletes the request the game left, if any.
pub fn take_request(dir: &Path) -> Option<(String, String)> {
    let path = dir.join(REQUEST_FILE);
    let text = std::fs::read_to_string(&path).ok()?;
    let _ = std::fs::remove_file(&path);
    parse_request(&text)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::SavedAccount;

    fn settings() -> Settings {
        let mut s = Settings::default();
        s.accounts = vec![
            SavedAccount { name: "Alex".into(), uuid: "aa".into(), refresh_token: "secret-token".into() },
            SavedAccount { name: "Steve".into(), uuid: "bb".into(), refresh_token: "other-secret".into() },
        ];
        s.active_account = "aa".into();
        s
    }

    #[test]
    fn the_game_learns_names_and_ids_but_no_token() {
        let json = accounts_json(&settings());
        assert!(!json.contains("secret"));
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["active"], "aa");
        assert_eq!(v["accounts"][1]["name"], "Steve");
        assert_eq!(v["can_switch"], true);
    }

    #[test]
    fn a_launcher_that_closes_at_start_cannot_switch() {
        let mut s = settings();
        s.after_start = "close".into();
        let v: serde_json::Value = serde_json::from_str(&accounts_json(&s)).unwrap();
        assert_eq!(v["can_switch"], false);
    }

    #[test]
    fn requests_are_read_and_bad_ones_refused() {
        assert_eq!(parse_request(r#"{"uuid":"bb","address":"mc.example.org:25565"}"#), Some(("bb".into(), "mc.example.org:25565".into())));
        assert_eq!(parse_request(r#"{"uuid":"bb"}"#), Some(("bb".into(), String::new())));
        assert_eq!(parse_request(r#"{"uuid":""}"#), None);
        assert_eq!(parse_request(r#"{"uuid":"bb","address":"a b"}"#), None);
        assert_eq!(parse_request("nonsense"), None);
    }

    #[test]
    fn a_request_is_taken_once() {
        let dir = std::env::temp_dir().join(format!("belumi-switch-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(REQUEST_FILE), r#"{"uuid":"bb","address":""}"#).unwrap();
        assert_eq!(take_request(&dir), Some(("bb".into(), String::new())));
        assert_eq!(take_request(&dir), None);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
