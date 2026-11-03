use serde_json::Value;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

pub const MIN_LIFETIME_SECS: u64 = 120;

#[derive(Debug, Clone)]
pub struct PrismAccount {
    pub name: String,
    pub uuid: String,
    pub access_token: String,
    pub expires_at: u64,
    pub modified: Option<SystemTime>,
}

pub fn candidate_paths(custom: &str) -> Vec<PathBuf> {
    let custom = custom.trim();
    if !custom.is_empty() {
        return vec![PathBuf::from(custom)];
    }
    let mut paths = Vec::new();
    let home = std::env::var_os("HOME").map(PathBuf::from);
    if let Some(data) = std::env::var_os("XDG_DATA_HOME").filter(|d| !d.is_empty()) {
        paths.push(PathBuf::from(data).join("PrismLauncher/accounts.json"));
    }
    if let Some(home) = home {
        paths.push(home.join(".local/share/PrismLauncher/accounts.json"));
        paths.push(
            home.join(".var/app/org.prismlauncher.PrismLauncher/data/PrismLauncher/accounts.json"),
        );
    }
    paths
}

pub fn load(custom_path: &str, now: u64) -> Result<PrismAccount, String> {
    let paths = candidate_paths(custom_path);
    let path = paths.iter().find(|p| p.is_file()).ok_or(
        "Prism Launcher's accounts.json was not found. Sign in with Prism first, or set the path.",
    )?;
    load_from(path, now)
}

pub fn load_from(path: &Path, now: u64) -> Result<PrismAccount, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| format!("Could not read {}: {e}", path.display()))?;
    let modified = std::fs::metadata(path).and_then(|m| m.modified()).ok();
    let mut account = parse(&text, now)?;
    account.modified = modified;
    Ok(account)
}

pub fn parse(text: &str, now: u64) -> Result<PrismAccount, String> {
    let root: Value = serde_json::from_str(text)
        .map_err(|_| "Prism's accounts.json is not valid JSON.".to_owned())?;
    let accounts = root["accounts"]
        .as_array()
        .ok_or("Prism's accounts.json has no accounts.")?;
    let microsoft = |a: &&Value| a["type"].as_str() == Some("MSA");
    let chosen = accounts
        .iter()
        .filter(microsoft)
        .find(|a| a["active"].as_bool() == Some(true))
        .or_else(|| accounts.iter().find(microsoft))
        .ok_or("Prism has no Microsoft account. Add one in Prism first.")?;
    let name = chosen["profile"]["name"]
        .as_str()
        .ok_or("Prism's account has no profile name.")?;
    let uuid = chosen["profile"]["id"]
        .as_str()
        .ok_or("Prism's account has no profile id.")?;
    let token = chosen["ygg"]["token"].as_str().filter(|t| !t.is_empty());
    let expires_at = chosen["ygg"]["exp"].as_u64().unwrap_or(0);
    let Some(token) = token else {
        return Err(format!(
            "Prism has no Minecraft session for {name} yet. Open Prism once so it signs in."
        ));
    };
    if expires_at < now + MIN_LIFETIME_SECS {
        return Err(format!(
            "Prism's session for {name} has expired. Open Prism Launcher once: it renews the session and this launcher picks it up automatically."
        ));
    }
    Ok(PrismAccount {
        name: name.to_owned(),
        uuid: uuid.to_owned(),
        access_token: token.to_owned(),
        expires_at,
        modified: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const NOW: u64 = 1_000_000;

    fn file(accounts: Value) -> String {
        json!({ "accounts": accounts, "formatVersion": 3 }).to_string()
    }
    fn msa(name: &str, active: bool, exp: u64) -> Value {
        json!({
            "type": "MSA", "active": active,
            "profile": { "id": format!("uuid-{name}"), "name": name },
            "ygg": { "token": format!("secret-{name}"), "exp": exp },
            "msa": { "refresh_token": "never-read" },
        })
    }

    #[test]
    fn takes_the_active_microsoft_account() {
        let text = file(json!([
            msa("First", false, NOW + 9999),
            msa("Second", true, NOW + 9999)
        ]));
        let a = parse(&text, NOW).unwrap();
        assert_eq!(
            (a.name.as_str(), a.uuid.as_str(), a.access_token.as_str()),
            ("Second", "uuid-Second", "secret-Second")
        );
        assert_eq!(a.expires_at, NOW + 9999);
    }

    #[test]
    fn falls_back_to_the_first_microsoft_account_and_skips_offline_ones() {
        let offline =
            json!({ "type": "Offline", "active": true, "profile": { "id": "x", "name": "Off" } });
        let a = parse(&file(json!([offline, msa("Only", false, NOW + 9999)])), NOW).unwrap();
        assert_eq!(a.name, "Only");
        let none = parse(&file(json!([offline])), NOW).unwrap_err();
        assert!(none.contains("no Microsoft account"));
    }

    #[test]
    fn an_expired_or_missing_session_asks_for_prism_and_never_leaks_the_token() {
        let expired = parse(&file(json!([msa("Old", true, NOW + 10)])), NOW).unwrap_err();
        assert!(expired.contains("has expired") && expired.contains("Old"));
        assert!(!expired.contains("secret"));
        let mut account = msa("New", true, 0);
        account["ygg"] = json!({});
        let missing = parse(&file(json!([account])), NOW).unwrap_err();
        assert!(missing.contains("no Minecraft session"));
    }

    #[test]
    fn reports_broken_files_without_their_content() {
        assert!(
            parse("{ nope secret", NOW)
                .unwrap_err()
                .contains("not valid JSON")
        );
        assert!(parse("{}", NOW).unwrap_err().contains("no accounts"));
    }

    #[test]
    fn reads_a_file_and_its_modification_time() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("accounts.json");
        std::fs::write(&path, file(json!([msa("Alex", true, NOW + 9999)]))).unwrap();
        let a = load_from(&path, NOW).unwrap();
        assert!(a.modified.is_some());
        let missing = load(dir.path().join("none.json").to_str().unwrap(), NOW).unwrap_err();
        assert!(missing.contains("not found"));
        assert_eq!(
            candidate_paths(" /x/y.json "),
            vec![PathBuf::from("/x/y.json")]
        );
    }
}
