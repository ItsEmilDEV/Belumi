//! Client plugins from the launcher: a list of plugins (an index a person or a community hosts), install with a checked
//! download, remove, switch off. A plugin is two files in `belumi-plugins/<name>/` of the game folder: `plugin.toml` and
//! `plugin.wasm`. The game reloads them by itself when they change.
use belumi_client_plugin_api::Manifest;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    io::Read,
    path::{Path, PathBuf},
    time::Duration,
};

pub const FOLDER: &str = "belumi-plugins";
const DISABLED: &str = ".disabled";
const MAX_MANIFEST: u64 = 64 * 1024;
const MAX_WASM: u64 = 8 * 1024 * 1024;

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct Entry {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub description: String,
    /// What the plugin asks for, shown before it is installed and checked against the downloaded manifest.
    #[serde(default)]
    pub permissions: Vec<String>,
    pub manifest_url: String,
    pub wasm_url: String,
    /// SHA-256 of `plugin.wasm`, in hex.
    pub wasm_sha256: String,
}

#[derive(Deserialize)]
struct Index {
    #[serde(default)]
    plugins: Vec<Entry>,
}

fn name_ok(name: &str) -> bool {
    !name.is_empty() && name.len() <= 32 && name.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_')
}

fn https(url: &str) -> bool {
    url.starts_with("https://") && !url.contains(char::is_whitespace) && url.len() <= 2048
}

/// The entries of an index; one with a bad name, a non-https link or a malformed hash is dropped.
pub fn parse_index(json: &str) -> Result<Vec<Entry>, String> {
    let index: Index = serde_json::from_str(json).map_err(|e| format!("The plugin list is not valid: {e}"))?;
    let mut seen = BTreeSet::new();
    Ok(index
        .plugins
        .into_iter()
        .filter(|e| {
            name_ok(&e.name)
                && https(&e.manifest_url)
                && https(&e.wasm_url)
                && e.wasm_sha256.len() == 64
                && e.wasm_sha256.bytes().all(|b| b.is_ascii_hexdigit())
                && seen.insert(e.name.clone())
        })
        .collect())
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Installed {
    pub name: String,
    pub version: String,
    pub enabled: bool,
}

/// The plugins in `<game dir>/belumi-plugins`.
pub fn installed(game_dir: &Path) -> Vec<Installed> {
    let Ok(entries) = std::fs::read_dir(game_dir.join(FOLDER)) else { return Vec::new() };
    let mut found: Vec<Installed> = entries
        .flatten()
        .filter(|e| e.path().is_dir())
        .filter_map(|e| {
            let manifest = Manifest::parse(&std::fs::read_to_string(e.path().join("plugin.toml")).ok()?).ok()?;
            Some(Installed { name: manifest.name, version: manifest.version, enabled: !e.path().join(DISABLED).exists() })
        })
        .collect();
    found.sort_by(|a, b| a.name.cmp(&b.name));
    found
}

fn folder(game_dir: &Path, name: &str) -> Option<PathBuf> {
    name_ok(name).then(|| game_dir.join(FOLDER).join(name))
}

pub fn remove(game_dir: &Path, name: &str) -> Result<(), String> {
    let dir = folder(game_dir, name).ok_or("That is not a plugin name.")?;
    std::fs::remove_dir_all(dir).map_err(|e| e.to_string())
}

pub fn set_enabled(game_dir: &Path, name: &str, enabled: bool) -> Result<(), String> {
    let marker = folder(game_dir, name).ok_or("That is not a plugin name.")?.join(DISABLED);
    if enabled {
        let _ = std::fs::remove_file(marker);
        Ok(())
    } else {
        std::fs::write(marker, b"").map_err(|e| e.to_string())
    }
}

/// Downloads and installs `entry`. `fetch(url, limit)` returns at most `limit` bytes of the file. Nothing is written until
/// both files passed: the manifest has to be the plugin's listed name, version and permissions, and the module the listed
/// hash and a WebAssembly header.
pub fn install(entry: &Entry, game_dir: &Path, fetch: &dyn Fn(&str, u64) -> Result<Vec<u8>, String>) -> Result<(), String> {
    let manifest_text = String::from_utf8(fetch(&entry.manifest_url, MAX_MANIFEST)?).map_err(|_| "The manifest is not text.")?;
    let manifest = Manifest::parse(&manifest_text).map_err(|e| e.to_string())?;
    if manifest.name != entry.name {
        return Err(format!("The manifest is for {}, not {}.", manifest.name, entry.name));
    }
    if manifest.version != entry.version {
        return Err(format!("The manifest is version {}, the list says {}.", manifest.version, entry.version));
    }
    let wanted: BTreeSet<String> = entry.permissions.iter().map(|p| p.to_ascii_lowercase()).collect();
    let asked: BTreeSet<String> = manifest.permissions.iter().map(|p| format!("{p:?}").to_ascii_lowercase()).collect();
    if wanted != asked {
        return Err("The plugin asks for other permissions than the list says; it was not installed.".to_owned());
    }
    let wasm = fetch(&entry.wasm_url, MAX_WASM)?;
    if !wasm.starts_with(b"\0asm") {
        return Err("The download is not a WebAssembly module.".to_owned());
    }
    let digest: String = Sha256::digest(&wasm).iter().map(|b| format!("{b:02x}")).collect();
    if !digest.eq_ignore_ascii_case(&entry.wasm_sha256) {
        return Err("The download does not match its checksum; it was not installed.".to_owned());
    }
    let dir = folder(game_dir, &entry.name).ok_or("That is not a plugin name.")?;
    let staging = dir.with_extension("installing");
    let _ = std::fs::remove_dir_all(&staging);
    std::fs::create_dir_all(&staging).map_err(|e| e.to_string())?;
    let written = std::fs::write(staging.join("plugin.toml"), manifest_text).and_then(|()| std::fs::write(staging.join("plugin.wasm"), wasm));
    if let Err(e) = written {
        let _ = std::fs::remove_dir_all(&staging);
        return Err(e.to_string());
    }
    // An update keeps the player's choice to have it off.
    if dir.join(DISABLED).exists() {
        let _ = std::fs::write(staging.join(DISABLED), b"");
    }
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::rename(&staging, &dir).map_err(|e| e.to_string())
}

/// A download with a size limit, for `install` and the list.
pub fn http_fetch(url: &str, limit: u64) -> Result<Vec<u8>, String> {
    if !https(url) {
        return Err("Only https links are used.".to_owned());
    }
    let response = ureq::get(url).timeout(Duration::from_secs(30)).call().map_err(|e| format!("Download failed: {e}"))?;
    let mut bytes = Vec::new();
    response.into_reader().take(limit + 1).read_to_end(&mut bytes).map_err(|e| format!("Download failed: {e}"))?;
    if bytes.len() as u64 > limit {
        return Err("The download is larger than allowed.".to_owned());
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    const WASM: &[u8] = b"\0asm\x01\0\0\0";

    fn sha(bytes: &[u8]) -> String {
        Sha256::digest(bytes).iter().map(|b| format!("{b:02x}")).collect()
    }

    fn entry(wasm: &[u8]) -> Entry {
        Entry {
            name: "compass".into(),
            version: "1.0".into(),
            description: String::new(),
            permissions: vec!["hud".into(), "chat".into()],
            manifest_url: "https://example.org/plugin.toml".into(),
            wasm_url: "https://example.org/plugin.wasm".into(),
            wasm_sha256: sha(wasm),
        }
    }

    const MANIFEST: &str = "name = \"compass\"\nversion = \"1.0\"\napi_version = 1\npermissions = [\"hud\", \"chat\"]\n";

    fn fetcher<'a>(manifest: &'a str, wasm: &'a [u8]) -> impl Fn(&str, u64) -> Result<Vec<u8>, String> + 'a {
        move |url, _| Ok(if url.ends_with(".toml") { manifest.as_bytes().to_vec() } else { wasm.to_vec() })
    }

    fn temp(tag: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("belumi-launcher-plugins-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn the_index_keeps_good_entries_and_drops_bad_ones() {
        let good = r#"{"name":"a-b_1","version":"1","manifest_url":"https://x/y.toml","wasm_url":"https://x/y.wasm","wasm_sha256":"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"}"#;
        let bad_name = good.replace("a-b_1", "Bad Name");
        let http = good.replace("https://x/y.wasm", "http://x/y.wasm");
        let short = good.replace("AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA", "abc");
        let json = format!(r#"{{"plugins":[{good},{bad_name},{http},{short},{good}]}}"#);
        let list = parse_index(&json).unwrap();
        assert_eq!(list.len(), 1, "a duplicate name is dropped too");
        assert!(parse_index("nonsense").is_err());
    }

    #[test]
    fn a_checked_plugin_is_installed_listed_switched_off_and_removed() {
        let game = temp("flow");
        install(&entry(WASM), &game, &fetcher(MANIFEST, WASM)).unwrap();
        assert_eq!(installed(&game), vec![Installed { name: "compass".into(), version: "1.0".into(), enabled: true }]);
        set_enabled(&game, "compass", false).unwrap();
        assert!(!installed(&game)[0].enabled);
        // An update keeps it switched off.
        install(&entry(WASM), &game, &fetcher(MANIFEST, WASM)).unwrap();
        assert!(!installed(&game)[0].enabled);
        set_enabled(&game, "compass", true).unwrap();
        assert!(installed(&game)[0].enabled);
        remove(&game, "compass").unwrap();
        assert!(installed(&game).is_empty());
        let _ = std::fs::remove_dir_all(&game);
    }

    #[test]
    fn a_wrong_checksum_name_version_or_permission_installs_nothing() {
        let game = temp("refuse");
        let mut wrong_hash = entry(WASM);
        wrong_hash.wasm_sha256 = sha(b"other");
        assert!(install(&wrong_hash, &game, &fetcher(MANIFEST, WASM)).unwrap_err().contains("checksum"));
        assert!(install(&entry(WASM), &game, &fetcher(&MANIFEST.replace("compass", "other"), WASM)).unwrap_err().contains("not compass"));
        assert!(install(&entry(WASM), &game, &fetcher(&MANIFEST.replace("1.0", "2.0"), WASM)).unwrap_err().contains("version"));
        assert!(install(&entry(WASM), &game, &fetcher(&MANIFEST.replace("\"hud\", ", ""), WASM)).unwrap_err().contains("permissions"));
        assert!(install(&entry(b"not wasm"), &game, &fetcher(MANIFEST, b"not wasm")).unwrap_err().contains("WebAssembly"));
        assert!(installed(&game).is_empty());
        assert!(!game.join(FOLDER).exists() || std::fs::read_dir(game.join(FOLDER)).unwrap().next().is_none());
        let _ = std::fs::remove_dir_all(&game);
    }

    #[test]
    fn names_cannot_leave_the_plugin_folder() {
        let game = temp("names");
        assert!(remove(&game, "../x").is_err());
        assert!(set_enabled(&game, "a/b", false).is_err());
        let mut sneaky = entry(WASM);
        sneaky.name = "../evil".into();
        assert!(install(&sneaky, &game, &fetcher(&MANIFEST.replace("compass", "../evil"), WASM)).is_err());
        let _ = std::fs::remove_dir_all(&game);
    }
}
