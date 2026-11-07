use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::cmp::Ordering;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering as Atomic};
use std::time::Duration;

pub const RELEASES_URL: &str = "https://api.github.com/repos/ItsEmilDEV/Belumi/releases?per_page=15";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Channel {
    Stable,
    Nightly,
}

impl Channel {
    pub fn parse(s: &str) -> Self {
        if s.trim().eq_ignore_ascii_case("nightly") {
            Channel::Nightly
        } else {
            Channel::Stable
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Channel::Stable => "stable",
            Channel::Nightly => "nightly",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct Release {
    #[serde(rename = "tag_name")]
    pub tag: String,
    #[serde(rename = "html_url")]
    pub url: String,
    #[serde(default)]
    pub prerelease: bool,
    #[serde(default)]
    pub draft: bool,
    #[serde(default)]
    pub assets: Vec<Asset>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct Asset {
    pub name: String,
    #[serde(rename = "browser_download_url")]
    pub url: String,
    #[serde(default)]
    pub size: u64,
}

/// Releases are only ever downloaded from Belumi's own GitHub releases.
const DOWNLOAD_PREFIX: &str = "https://github.com/ItsEmilDEV/Belumi/releases/download/";

/// How far a download is: bytes so far and bytes expected (0 while unknown).
#[derive(Default)]
pub struct Progress {
    pub done: AtomicU64,
    pub total: AtomicU64,
}

impl Progress {
    /// 0 to 100, or `None` while the size is not known.
    pub fn percent(&self) -> Option<u32> {
        let total = self.total.load(Atomic::Relaxed);
        (total > 0).then(|| (self.done.load(Atomic::Relaxed).min(total) * 100 / total) as u32)
    }
}

/// The archive of `release` for this system: `*-linux-<arch>.tar.gz` or `*-windows-x64.zip`.
pub fn platform_asset<'a>(release: &'a Release, os: &str, arch: &str) -> Option<&'a Asset> {
    let (needle, ext) = match (os, arch) {
        ("windows", "x86_64") => ("windows-x64".to_owned(), ".zip"),
        ("linux", arch) => (format!("linux-{arch}"), ".tar.gz"),
        _ => return None,
    };
    release
        .assets
        .iter()
        .find(|a| a.name.ends_with(ext) && a.name.contains(&needle) && !a.name.contains("setup") && a.url.starts_with(DOWNLOAD_PREFIX))
}

/// The first word of a `sha256sum`-style line, when it is a SHA-256 digest.
pub fn parse_checksum(text: &str) -> Option<String> {
    let word = text.split_whitespace().next()?.to_ascii_lowercase();
    (word.len() == 64 && word.bytes().all(|b| b.is_ascii_hexdigit())).then_some(word)
}

fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(15))
        .timeout_read(Duration::from_secs(60))
        .redirects(5)
        .build()
}

fn download(url: &str, to: &Path, progress: &Progress) -> Result<String, String> {
    if !url.starts_with(DOWNLOAD_PREFIX) {
        return Err("The download is not on Belumi's GitHub releases.".into());
    }
    let response = agent()
        .get(url)
        .set("User-Agent", "belumi-launcher")
        .call()
        .map_err(|e| format!("Download failed: {e}"))?;
    progress.done.store(0, Atomic::Relaxed);
    progress
        .total
        .store(response.header("Content-Length").and_then(|v| v.parse().ok()).unwrap_or(0), Atomic::Relaxed);
    let mut reader = response.into_reader();
    let mut file = std::fs::File::create(to).map_err(|e| format!("Could not write {}: {e}", to.display()))?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; 64 * 1024];
    loop {
        let n = reader.read(&mut buffer).map_err(|e| format!("Download failed: {e}"))?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
        file.write_all(&buffer[..n]).map_err(|e| format!("Could not write {}: {e}", to.display()))?;
        progress.done.fetch_add(n as u64, Atomic::Relaxed);
    }
    Ok(hasher.finalize().iter().map(|b| format!("{b:02x}")).collect())
}

fn find_file(dir: &Path, name: &str) -> Option<PathBuf> {
    for entry in std::fs::read_dir(dir).ok()?.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if let Some(found) = find_file(&path, name) {
                return Some(found);
            }
        } else if path.file_name().is_some_and(|n| n == name) {
            return Some(path);
        }
    }
    None
}

fn exe_name(base: &str) -> String {
    format!("{base}{}", std::env::consts::EXE_SUFFIX)
}

/// Puts the new launcher and game next to the running ones. The old files stay as `*.old` (a running
/// program on Windows cannot be overwritten, only renamed); a failure puts everything back.
pub fn replace_binaries(extracted: &Path, install_dir: &Path) -> Result<(), String> {
    let mut done: Vec<(PathBuf, PathBuf)> = Vec::new();
    let result = (|| {
        for base in ["horizon-client", "belumi-launcher"] {
            let name = exe_name(base);
            let new = find_file(extracted, &name).ok_or_else(|| format!("The download has no {name}."))?;
            let target = install_dir.join(&name);
            let staged = install_dir.join(format!("{name}.new"));
            let old = install_dir.join(format!("{name}.old"));
            std::fs::copy(&new, &staged).map_err(|e| format!("Could not write {}: {e}", staged.display()))?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = std::fs::set_permissions(&staged, std::fs::Permissions::from_mode(0o755));
            }
            let _ = std::fs::remove_file(&old);
            if target.exists() {
                std::fs::rename(&target, &old).map_err(|e| format!("Could not replace {}: {e}", target.display()))?;
            }
            done.push((target.clone(), old));
            std::fs::rename(&staged, &target).map_err(|e| format!("Could not replace {}: {e}", target.display()))?;
        }
        Ok(())
    })();
    if result.is_err() {
        for (target, old) in done.iter().rev() {
            if old.exists() {
                let _ = std::fs::remove_file(target);
                let _ = std::fs::rename(old, target);
            }
        }
    }
    result
}

fn ensure_writable(dir: &Path) -> Result<(), String> {
    let probe = dir.join(".belumi-update-test");
    std::fs::write(&probe, b"").map_err(|_| {
        format!(
            "Belumi cannot change {}. Install the new version with its installer or your package manager instead.",
            dir.display()
        )
    })?;
    let _ = std::fs::remove_file(probe);
    Ok(())
}

/// Downloads `release` for this system, checks its SHA-256 against the release's `.sha256` file and
/// replaces the launcher and the game next to the running launcher. The new version runs after a restart.
pub fn install(release: &Release, progress: &Progress) -> Result<(), String> {
    let asset = platform_asset(release, std::env::consts::OS, std::env::consts::ARCH)
        .ok_or("This release has no download for this system. Open the release page instead.")?;
    let sum_name = format!("{}.sha256", asset.name);
    let sum = release
        .assets
        .iter()
        .find(|a| a.name == sum_name && a.url.starts_with(DOWNLOAD_PREFIX))
        .ok_or("This release has no checksum file, so Belumi will not install it.")?;
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let dir = exe.parent().ok_or("Cannot find the launcher's folder.")?.to_path_buf();
    ensure_writable(&dir)?;

    let work = std::env::temp_dir().join(format!("belumi-update-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&work);
    std::fs::create_dir_all(&work).map_err(|e| e.to_string())?;
    let result = (|| {
        let sum_file = work.join("checksum");
        download(&sum.url, &sum_file, &Progress::default())?;
        let expected = parse_checksum(&std::fs::read_to_string(&sum_file).map_err(|e| e.to_string())?)
            .ok_or("The release's checksum file is not a SHA-256 sum.")?;
        let archive = work.join(&asset.name);
        let actual = download(&asset.url, &archive, progress)?;
        if actual != expected {
            return Err("The download is damaged (its checksum does not match). Try again.".to_owned());
        }
        let extracted = work.join("extracted");
        std::fs::create_dir_all(&extracted).map_err(|e| e.to_string())?;
        let status = std::process::Command::new("tar")
            .arg("-xf")
            .arg(&archive)
            .arg("-C")
            .arg(&extracted)
            .status()
            .map_err(|e| format!("Could not run tar to unpack the update: {e}"))?;
        if !status.success() {
            return Err("Could not unpack the update.".into());
        }
        replace_binaries(&extracted, &dir)
    })();
    let _ = std::fs::remove_dir_all(&work);
    result
}

/// At start: removes the `*.old` files an earlier update left next to the launcher.
pub fn clean_up_old() {
    let Some(dir) = std::env::current_exe().ok().and_then(|e| e.parent().map(Path::to_path_buf)) else {
        return;
    };
    for base in ["horizon-client", "belumi-launcher"] {
        let _ = std::fs::remove_file(dir.join(format!("{}.old", exe_name(base))));
    }
}

/// Starts the launcher again (the new one after an update) and ends this one.
pub fn restart() -> ! {
    if let Ok(exe) = std::env::current_exe() {
        let _ = std::process::Command::new(exe).args(std::env::args().skip(1)).spawn();
    }
    std::process::exit(0)
}

pub fn parse_releases(json: &str) -> Vec<Release> {
    serde_json::from_str(json).unwrap_or_default()
}

#[derive(Debug, PartialEq, Eq)]
struct Version {
    core: [u64; 3],
    pre: Option<String>,
}

fn parse_version(s: &str) -> Option<Version> {
    let s = s.trim().trim_start_matches('v');
    let s = s.split_once('+').map_or(s, |(v, _)| v);
    let (core, pre) = match s.split_once('-') {
        Some((c, p)) => (c, Some(p.to_owned())),
        None => (s, None),
    };
    let mut parts = core.split('.');
    let mut out = [0u64; 3];
    for slot in &mut out {
        *slot = parts.next()?.parse().ok()?;
    }
    if parts.next().is_some() {
        return None;
    }
    Some(Version { core: out, pre })
}

fn compare_pre(a: &str, b: &str) -> Ordering {
    let (mut x, mut y) = (a.split('.'), b.split('.'));
    loop {
        match (x.next(), y.next()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(p), Some(q)) => {
                let o = match (p.parse::<u64>(), q.parse::<u64>()) {
                    (Ok(m), Ok(n)) => m.cmp(&n),
                    (Ok(_), Err(_)) => Ordering::Less,
                    (Err(_), Ok(_)) => Ordering::Greater,
                    (Err(_), Err(_)) => p.cmp(q),
                };
                if o != Ordering::Equal {
                    return o;
                }
            }
        }
    }
}

pub fn is_newer(candidate: &str, current: &str) -> bool {
    let (Some(c), Some(n)) = (parse_version(candidate), parse_version(current)) else {
        return false;
    };
    match c.core.cmp(&n.core) {
        Ordering::Greater => true,
        Ordering::Less => false,
        Ordering::Equal => match (&c.pre, &n.pre) {
            (None, Some(_)) => true,
            (Some(a), Some(b)) => compare_pre(a, b) == Ordering::Greater,
            _ => false,
        },
    }
}

pub fn pick(channel: Channel, releases: &[Release], current: &str) -> Option<Release> {
    releases
        .iter()
        .filter(|r| !r.draft && (channel == Channel::Nightly || !r.prerelease))
        .filter(|r| is_newer(&r.tag, current))
        .fold(None::<&Release>, |best, r| match best {
            Some(b) if !is_newer(&r.tag, &b.tag) => Some(b),
            _ => Some(r),
        })
        .cloned()
}

pub fn check(channel: Channel, current: &str) -> Result<Option<Release>, String> {
    let body = ureq::get(RELEASES_URL)
        .set("User-Agent", "belumi-launcher")
        .set("Accept", "application/vnd.github+json")
        .timeout(std::time::Duration::from_secs(10))
        .call()
        .map_err(|e| format!("update check failed: {e}"))?
        .into_string()
        .map_err(|e| format!("update check failed: {e}"))?;
    Ok(pick(channel, &parse_releases(&body), current))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn release(tag: &str, pre: bool) -> Release {
        Release {
            tag: tag.into(),
            url: format!("https://example.org/{tag}"),
            prerelease: pre,
            draft: false,
            assets: Vec::new(),
        }
    }

    #[test]
    fn versions_compare_by_number_not_by_text() {
        assert!(is_newer("v0.10.0", "0.9.9"));
        assert!(is_newer("1.0.0", "0.99.99"));
        assert!(!is_newer("0.1.0", "0.1.0"));
        assert!(!is_newer("0.0.9", "0.1.0"));
        assert!(!is_newer("garbage", "0.1.0"));
        assert!(!is_newer("0.1.0", "garbage"));
        assert!(!is_newer("1.2", "0.1.0"));
    }

    #[test]
    fn a_release_beats_its_prereleases_and_prereleases_order_by_segment() {
        assert!(is_newer("0.2.0", "0.2.0-beta.3"));
        assert!(!is_newer("0.2.0-beta.3", "0.2.0"));
        assert!(is_newer("0.2.0-beta.10", "0.2.0-beta.9"));
        assert!(is_newer("0.2.0-rc.1", "0.2.0-beta.9"));
        assert!(is_newer("0.2.0-beta.1.1", "0.2.0-beta.1"));
    }

    #[test]
    fn stable_ignores_prereleases_and_drafts_nightly_takes_them() {
        let list = vec![
            release("v0.3.0-nightly.5", true),
            release("v0.2.0", false),
            Release { draft: true, ..release("v9.0.0", false) },
            release("v0.1.0", false),
        ];
        assert_eq!(pick(Channel::Stable, &list, "0.1.0").unwrap().tag, "v0.2.0");
        assert_eq!(pick(Channel::Nightly, &list, "0.1.0").unwrap().tag, "v0.3.0-nightly.5");
        assert_eq!(pick(Channel::Stable, &list, "0.2.0"), None);
        assert_eq!(pick(Channel::Nightly, &list, "0.3.0-nightly.5"), None);
    }

    #[test]
    fn github_release_lists_parse_and_garbage_gives_nothing() {
        let json = r#"[{"tag_name":"v1.0.0","html_url":"https://github.com/x/y/releases/tag/v1.0.0","prerelease":false,"draft":false,"name":"x","assets":[]}]"#;
        let r = parse_releases(json);
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].tag, "v1.0.0");
        assert!(parse_releases("{\"message\":\"rate limited\"}").is_empty());
        assert!(parse_releases("").is_empty());
    }

    #[test]
    fn channels_parse_leniently() {
        assert_eq!(Channel::parse("Nightly"), Channel::Nightly);
        assert_eq!(Channel::parse("stable"), Channel::Stable);
        assert_eq!(Channel::parse("???"), Channel::Stable);
        assert_eq!(Channel::Nightly.name(), "nightly");
    }

    fn asset(name: &str) -> Asset {
        Asset { name: name.into(), url: format!("{DOWNLOAD_PREFIX}v1.0.0/{name}"), size: 1 }
    }

    #[test]
    fn the_download_for_this_system_is_picked_and_the_installer_is_not() {
        let mut r = release("v1.0.0", false);
        r.assets = ["belumi-1.0.0-linux-x86_64.tar.gz", "belumi-1.0.0-linux-x86_64.tar.gz.sha256", "belumi-1.0.0-windows-x64.zip", "belumi-1.0.0-windows-x64-setup.exe"]
            .into_iter()
            .map(asset)
            .collect();
        assert_eq!(platform_asset(&r, "linux", "x86_64").unwrap().name, "belumi-1.0.0-linux-x86_64.tar.gz");
        assert_eq!(platform_asset(&r, "windows", "x86_64").unwrap().name, "belumi-1.0.0-windows-x64.zip");
        assert!(platform_asset(&r, "linux", "aarch64").is_none());
        assert!(platform_asset(&r, "windows", "aarch64").is_none());
        assert!(platform_asset(&r, "macos", "aarch64").is_none());
        // Anything outside Belumi's releases is never downloaded.
        r.assets[0].url = "https://example.org/belumi-1.0.0-linux-x86_64.tar.gz".into();
        assert!(platform_asset(&r, "linux", "x86_64").is_none());
    }

    #[test]
    fn checksums_are_read_from_sha256sum_lines_only() {
        let digest = "a".repeat(64);
        assert_eq!(parse_checksum(&format!("{digest}  belumi.tar.gz\n")).unwrap(), digest);
        assert_eq!(parse_checksum(&digest.to_uppercase()).unwrap(), digest);
        assert!(parse_checksum("abc  file").is_none());
        assert!(parse_checksum("").is_none());
        assert!(parse_checksum(&format!("{}zz", "a".repeat(62))).is_none());
    }

    #[test]
    fn the_new_programs_replace_the_old_ones_and_a_failure_puts_them_back() {
        let dir = tempfile::tempdir().unwrap();
        let (install, new) = (dir.path().join("install"), dir.path().join("new"));
        std::fs::create_dir_all(new.join("pkg/bin")).unwrap();
        std::fs::create_dir_all(&install).unwrap();
        for base in ["horizon-client", "belumi-launcher"] {
            std::fs::write(install.join(exe_name(base)), "old").unwrap();
        }
        std::fs::write(new.join("pkg/bin").join(exe_name("horizon-client")), "new client").unwrap();
        // No launcher in the download: nothing is changed.
        assert!(replace_binaries(&new, &install).is_err());
        assert_eq!(std::fs::read_to_string(install.join(exe_name("horizon-client"))).unwrap(), "old");
        assert_eq!(std::fs::read_to_string(install.join(exe_name("belumi-launcher"))).unwrap(), "old");
        std::fs::write(new.join("pkg/bin").join(exe_name("belumi-launcher")), "new launcher").unwrap();
        replace_binaries(&new, &install).unwrap();
        assert_eq!(std::fs::read_to_string(install.join(exe_name("horizon-client"))).unwrap(), "new client");
        assert_eq!(std::fs::read_to_string(install.join(exe_name("belumi-launcher"))).unwrap(), "new launcher");
        assert_eq!(std::fs::read_to_string(install.join(format!("{}.old", exe_name("belumi-launcher")))).unwrap(), "old");
    }

    #[test]
    fn progress_is_a_percentage_once_the_size_is_known() {
        let p = Progress::default();
        assert_eq!(p.percent(), None);
        p.total.store(200, Atomic::Relaxed);
        p.done.store(50, Atomic::Relaxed);
        assert_eq!(p.percent(), Some(25));
        p.done.store(500, Atomic::Relaxed);
        assert_eq!(p.percent(), Some(100));
    }
}
