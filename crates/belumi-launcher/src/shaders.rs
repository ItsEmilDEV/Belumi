//! The Solas shader pack, fetched from Modrinth when the player says so. Solas is by Septonious and "All Rights Reserved":
//! Belumi never ships it, it only downloads it from the author's official source, as other launchers do. The game picks a
//! Solas pack by itself the first time it finds one in the game folder's `shaderpacks`.
use serde::Deserialize;
use sha2::{Digest, Sha512};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

/// The project's page, for the credit and for the "open" link.
pub const PAGE: &str = "https://modrinth.com/shader/solas-shader";
const VERSIONS_URL: &str = "https://api.modrinth.com/v2/project/solas-shader/version";
/// Files are only taken from Modrinth's own download host.
const CDN_PREFIX: &str = "https://cdn.modrinth.com/data/";
/// The game version the pack should support.
const GAME_VERSION: &str = "26.3";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pack {
    pub version: String,
    pub file: String,
    pub url: String,
    pub size: u64,
    pub sha512: String,
}

#[derive(Deserialize)]
struct Version {
    version_number: String,
    #[serde(default)]
    version_type: String,
    #[serde(default)]
    loaders: Vec<String>,
    #[serde(default)]
    game_versions: Vec<String>,
    #[serde(default)]
    files: Vec<File>,
}

#[derive(Deserialize)]
struct File {
    filename: String,
    url: String,
    #[serde(default)]
    primary: bool,
    #[serde(default)]
    size: u64,
    #[serde(default)]
    hashes: std::collections::BTreeMap<String, String>,
}

/// A file name that stays in the folder it is written to.
fn safe_name(name: &str) -> bool {
    name.to_ascii_lowercase().ends_with(".zip")
        && !name.contains("..")
        && name.chars().all(|c| c.is_ascii_alphanumeric() || " ._-+()".contains(c))
}

/// The newest release for the game version (else the newest release) from Modrinth's version list, newest first.
pub fn pick(json: &str) -> Option<Pack> {
    let versions: Vec<Version> = serde_json::from_str(json).ok()?;
    let usable = |v: &&Version| {
        v.version_type == "release" && v.loaders.iter().any(|l| l == "iris" || l == "optifine")
    };
    let chosen = versions
        .iter()
        .filter(usable)
        .find(|v| v.game_versions.iter().any(|g| g == GAME_VERSION))
        .or_else(|| versions.iter().find(usable))?;
    let file = chosen.files.iter().find(|f| f.primary).or_else(|| chosen.files.first())?;
    let sha512 = file.hashes.get("sha512")?.to_ascii_lowercase();
    (file.url.starts_with(CDN_PREFIX) && safe_name(&file.filename) && sha512.len() == 128).then(|| Pack {
        version: chosen.version_number.clone(),
        file: file.filename.clone(),
        url: file.url.clone(),
        size: file.size,
        sha512,
    })
}

/// A Solas pack already in `dir` (the game folder's `shaderpacks`).
pub fn installed(dir: &Path) -> Option<String> {
    std::fs::read_dir(dir)
        .ok()?
        .flatten()
        .filter_map(|e| e.file_name().into_string().ok())
        .find(|n| {
            let l = n.to_ascii_lowercase();
            l.starts_with("solas") && l.ends_with(".zip")
        })
}

pub fn dir_of(game_dir: &Path) -> PathBuf {
    game_dir.join("shaderpacks")
}

fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(15))
        .timeout_read(Duration::from_secs(60))
        .build()
}

fn user_agent() -> String {
    format!("ItsEmilDEV/Belumi/{} (https://github.com/ItsEmilDEV/Belumi)", env!("CARGO_PKG_VERSION"))
}

/// Asks Modrinth for the pack, downloads it into `dir`, checks size and SHA-512 and returns the file's name.
pub fn download(dir: &Path) -> Result<String, String> {
    let list = agent()
        .get(VERSIONS_URL)
        .set("User-Agent", &user_agent())
        .call()
        .map_err(|e| format!("Could not ask Modrinth: {e}"))?
        .into_string()
        .map_err(|e| e.to_string())?;
    let pack = pick(&list).ok_or("Modrinth has no Solas release this launcher can use.")?;
    std::fs::create_dir_all(dir).map_err(|e| format!("Could not create {}: {e}", dir.display()))?;
    let target = dir.join(&pack.file);
    let part = dir.join(format!("{}.part", pack.file));
    let result = (|| {
        let response = agent()
            .get(&pack.url)
            .set("User-Agent", &user_agent())
            .call()
            .map_err(|e| format!("Download failed: {e}"))?;
        let mut reader = response.into_reader();
        let mut file = std::fs::File::create(&part).map_err(|e| format!("Could not write {}: {e}", part.display()))?;
        let mut hasher = Sha512::new();
        let mut buffer = vec![0u8; 64 * 1024];
        let mut total = 0u64;
        loop {
            let n = reader.read(&mut buffer).map_err(|e| format!("Download failed: {e}"))?;
            if n == 0 {
                break;
            }
            total += n as u64;
            if total > 64 * 1024 * 1024 {
                return Err("The download is far bigger than a shader pack; stopped.".to_owned());
            }
            hasher.update(&buffer[..n]);
            file.write_all(&buffer[..n]).map_err(|e| format!("Could not write {}: {e}", part.display()))?;
        }
        let digest: String = hasher.finalize().iter().map(|b| format!("{b:02x}")).collect();
        if digest != pack.sha512 || (pack.size > 0 && total != pack.size) {
            return Err("The download is damaged (its checksum does not match). Try again.".to_owned());
        }
        drop(file);
        std::fs::rename(&part, &target).map_err(|e| format!("Could not save {}: {e}", target.display()))?;
        Ok(pack.file.clone())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&part);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(version: &str, kind: &str, games: &[&str], loaders: &[&str], file: &str, url: &str, sha: &str) -> String {
        format!(
            r#"{{"version_number":"{version}","version_type":"{kind}","game_versions":{games:?},"loaders":{loaders:?},"files":[{{"filename":"{file}","url":"{url}","primary":true,"size":10,"hashes":{{"sha512":"{sha}","sha1":"x"}}}}]}}"#
        )
    }

    #[test]
    fn the_newest_release_for_the_game_version_is_chosen() {
        let sha = "a".repeat(128);
        let cdn = "https://cdn.modrinth.com/data/X/versions/Y/";
        let list = format!(
            "[{},{},{}]",
            entry("4.0-beta", "beta", &["26.3"], &["iris"], "Solas Shader V4.0b.zip", &format!("{cdn}a.zip"), &sha),
            entry("3.9", "release", &["26.2", "26.3"], &["iris", "optifine"], "Solas Shader V3.9.zip", &format!("{cdn}b.zip"), &sha),
            entry("3.8", "release", &["26.2"], &["iris"], "Solas Shader V3.8.zip", &format!("{cdn}c.zip"), &sha),
        );
        let pack = pick(&list).unwrap();
        assert_eq!((pack.version.as_str(), pack.file.as_str()), ("3.9", "Solas Shader V3.9.zip"));
        // Without a release for 26.3 the newest release still counts.
        let older = format!("[{}]", entry("3.8", "release", &["26.2"], &["iris"], "Solas Shader V3.8.zip", &format!("{cdn}c.zip"), &sha));
        assert_eq!(pick(&older).unwrap().version, "3.8");
        assert!(pick("[]").is_none() && pick("garbage").is_none());
    }

    #[test]
    fn only_modrinths_own_host_and_plain_file_names_are_accepted() {
        let sha = "b".repeat(128);
        let good = "https://cdn.modrinth.com/data/X/versions/Y/p.zip";
        for (file, url) in [
            ("Solas Shader V3.9.zip", "https://example.org/p.zip"),
            ("../evil.zip", good),
            ("a/b.zip", good),
            ("solas.exe", good),
            ("solas.zip.exe", good),
        ] {
            let list = format!("[{}]", entry("3.9", "release", &["26.3"], &["iris"], file, url, &sha));
            assert!(pick(&list).is_none(), "{file} {url}");
        }
        let short = format!("[{}]", entry("3.9", "release", &["26.3"], &["iris"], "p.zip", good, "abc"));
        assert!(pick(&short).is_none(), "a checksum is required");
    }

    #[test]
    fn an_installed_solas_pack_is_found_by_its_name() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(installed(dir.path()), None);
        std::fs::write(dir.path().join("Other Pack.zip"), "").unwrap();
        assert_eq!(installed(dir.path()), None);
        std::fs::write(dir.path().join("Solas Shader V3.9.zip"), "").unwrap();
        assert_eq!(installed(dir.path()).as_deref(), Some("Solas Shader V3.9.zip"));
    }

    /// Needs the network: `cargo test -p belumi-launcher -- --ignored real_download`.
    #[test]
    #[ignore]
    fn real_download_gets_a_checked_zip_from_modrinth() {
        let dir = tempfile::tempdir().unwrap();
        let name = download(dir.path()).unwrap();
        assert!(name.to_ascii_lowercase().starts_with("solas") && name.ends_with(".zip"), "{name}");
        let size = std::fs::metadata(dir.path().join(&name)).unwrap().len();
        assert!(size > 100_000, "{size}");
        assert_eq!(installed(dir.path()).as_deref(), Some(name.as_str()));
        assert!(!dir.path().join(format!("{name}.part")).exists());
    }
}
