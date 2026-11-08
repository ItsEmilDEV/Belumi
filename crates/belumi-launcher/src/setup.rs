//! Getting the game ready on a computer that has only Belumi: the game files come from Mojang's own servers (see `mojang.rs`),
//! then the preparer (`horizon-prepare`, linked in) turns them into the resources the game reads. What ships with Belumi is only
//! the small `horizon-files` folder: `research-lock.json` (the pinned versions and checksums) and the sources of Horizon's own
//! exporters, which run on Mojang's Java. Everything is kept in Belumi's data folder, so a later run goes on where one stopped.
use crate::mojang::{self, Pins};
use serde_json::Value;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::time::Duration;

/// The exporters' Java sources, relative to the bundle and to the preparer's root.
const SOURCES: &str = "mods/horizon/src/test/java";
/// Hosts the files may come from: Mojang's.
const HOSTS: [&str; 6] = [
    "piston-meta.mojang.com",
    "piston-data.mojang.com",
    "launchermeta.mojang.com",
    "launcher.mojang.com",
    "libraries.minecraft.net",
    "resources.download.minecraft.net",
];

/// A folder that is a bundle: it has the lock and the exporter sources.
fn is_bundle(dir: &Path) -> bool {
    dir.join("research-lock.json").is_file() && dir.join(SOURCES).join("dev/horizonlab/parity").is_dir()
}

/// The `horizon-files` shipped with this install (next to the launcher, in `share/belumi`), or a checkout while developing.
pub fn find_bundle() -> Option<PathBuf> {
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Some(dir) = std::env::var_os("BELUMI_HORIZON_FILES") {
        candidates.push(dir.into());
    }
    if let Some(exe) = std::env::current_exe().ok().and_then(|e| e.parent().map(Path::to_path_buf)) {
        candidates.push(exe.join("horizon-files"));
        candidates.push(exe.join("../share/belumi/horizon-files"));
    }
    candidates.push("/usr/share/belumi/horizon-files".into());
    candidates.push("/usr/local/share/belumi/horizon-files".into());
    let repo = std::env::var_os("BELUMI_REPO").map(PathBuf::from).or_else(|| std::env::current_dir().ok());
    candidates.extend(repo.map(|r| r.join("vendor/horizon")));
    candidates.into_iter().find(|d| is_bundle(d))
}

/// Where the preparer works and writes: `<data>/prepare`.
pub fn root(data_dir: &Path) -> PathBuf {
    data_dir.join("prepare")
}

fn same(a: &Path, b: &Path) -> bool {
    match (fs::metadata(a), fs::metadata(b)) {
        (Ok(x), Ok(y)) if x.len() == y.len() => fs::read(a).ok() == fs::read(b).ok(),
        _ => false,
    }
}

fn copy_tree(from: &Path, to: &Path) -> Result<(), String> {
    let error = |p: &Path, e: std::io::Error| format!("{}: {e}", p.display());
    fs::create_dir_all(to).map_err(|e| error(to, e))?;
    for entry in fs::read_dir(from).map_err(|e| error(from, e))?.flatten() {
        let (src, dst) = (entry.path(), to.join(entry.file_name()));
        if src.is_dir() {
            copy_tree(&src, &dst)?;
        } else if !same(&src, &dst) {
            fs::copy(&src, &dst).map_err(|e| error(&dst, e))?;
        }
    }
    Ok(())
}

/// Puts the bundle into the preparer's root: the lock, the exporter sources and, when the bundle has them, compiled exporters.
fn sync(bundle: &Path, root: &Path) -> Result<(), String> {
    fs::create_dir_all(root).map_err(|e| format!("{}: {e}", root.display()))?;
    if !same(&bundle.join("research-lock.json"), &root.join("research-lock.json")) {
        fs::copy(bundle.join("research-lock.json"), root.join("research-lock.json")).map_err(|e| e.to_string())?;
    }
    copy_tree(&bundle.join(SOURCES), &root.join(SOURCES))?;
    if bundle.join("exporters/manifest.json").is_file() {
        copy_tree(&bundle.join("exporters"), &root.join("exporters"))?;
    }
    Ok(())
}

fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new().timeout_connect(Duration::from_secs(15)).timeout_read(Duration::from_secs(60)).redirects(3).build()
}

/// Whether a URL is on one of Mojang's hosts, over https.
fn allowed(url: &str) -> bool {
    url.strip_prefix("https://").and_then(|rest| rest.split('/').next()).is_some_and(|host| HOSTS.contains(&host))
}

fn fetch(url: &str) -> Result<Vec<u8>, String> {
    if !allowed(url) {
        return Err(format!("{url} is not on a Mojang server"));
    }
    let response = agent().get(url).call().map_err(|e| format!("{url}: {e}"))?;
    let mut body = Vec::new();
    response.into_reader().take(1 << 31).read_to_end(&mut body).map_err(|e| format!("{url}: {e}"))?;
    Ok(body)
}

fn mojang_os() -> &'static str {
    if cfg!(windows) {
        "windows"
    } else if cfg!(target_os = "macos") {
        "osx"
    } else {
        "linux"
    }
}

fn read_json(path: &Path) -> Result<Value, String> {
    let text = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))
}

/// The Java for the preparer: one that is installed, else Mojang's own runtime (a whole JDK), downloaded.
fn java(major: i64, compiler: bool, minecraft: &Path, manifest: &Value, cancel: &AtomicBool, log: &dyn Fn(String)) -> Result<PathBuf, String> {
    let find = if compiler { horizon_prepare::java::find } else { horizon_prepare::java::find_runtime };
    let runtime = minecraft.join("runtime");
    let mut known = horizon_prepare::java::minecraft_runtimes(&runtime);
    if let Ok(jdk) = find(major, &known) {
        return Ok(jdk.home());
    }
    let component = manifest["javaVersion"]["component"].as_str().ok_or("The version file names no Java runtime")?;
    let home = mojang::java_runtime(component, &runtime, 8, &fetch, cancel, log)?;
    known.push(home);
    find(major, &known).map(|jdk| jdk.home())
}

/// Downloads the game from Mojang and prepares it; returns the resources folder. `log` gets a line for each step.
pub fn run(data_dir: &Path, cancel: &AtomicBool, log: &dyn Fn(String)) -> Result<PathBuf, String> {
    let bundle = find_bundle().ok_or("Belumi's horizon-files folder was not found next to the program. Reinstall Belumi.")?;
    let root = root(data_dir);
    sync(&bundle, &root)?;
    let lock = read_json(&root.join("research-lock.json"))?;
    let pins = Pins::read(&lock).ok_or("research-lock.json does not pin a Minecraft version")?;
    let major = lock["minecraft"]["java_major"].as_i64().unwrap_or(25);
    let minecraft = data_dir.join("minecraft");
    mojang::CANCEL.store(false, std::sync::atomic::Ordering::Relaxed);
    log(format!("Step 1 of 4: downloading Minecraft {} from Mojang", pins.version));
    mojang::download(&pins, &minecraft, mojang_os(), 8, &fetch, cancel, log)?;
    let manifest = read_json(&minecraft.join("versions").join(&pins.version).join(format!("{}.json", pins.version)))?;
    log("Step 2 of 4: Java".into());
    let compiler = !horizon_prepare::exporters::available(&root);
    let java_home = java(major, compiler, &minecraft, &manifest, cancel, log)?;
    let jar = data_dir.join("server-jars").join(format!("{}.jar", pins.version));
    if mojang::sha1_file(&jar).as_deref() != Some(pins.server_sha1.as_str()) {
        let server = &manifest["downloads"]["server"];
        let url = server["url"].as_str().ok_or("The version file has no server download")?;
        log(format!("Downloading the {} server jar from Mojang", pins.version));
        mojang::write_file(&jar, &mojang::fetch_checked(&fetch, url, &pins.server_sha1)?)?;
    }
    log("Step 3 of 4: preparing the game's resources (a few minutes the first time)".into());
    let options = horizon_prepare::Options {
        root: root.clone(),
        import: true,
        minecraft_dir: Some(minecraft.clone()),
        server_jar: Some(jar),
        java_home: Some(java_home),
    };
    let prepared = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| horizon_prepare::prepare(&options, &|l| log(l.to_string()))))
        .unwrap_or_else(|_| Err("The preparation stopped unexpectedly".into()))?;
    log("Step 4 of 4: the sounds".into());
    if let Err(e) = horizon_prepare::sounds(&root, Some(&minecraft), &|l| log(l.to_string())) {
        log(format!("No sound (the game will be silent): {e}"));
    }
    Ok(prepared)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fake_bundle(dir: &Path) {
        fs::create_dir_all(dir.join(SOURCES).join("dev/horizonlab/parity")).unwrap();
        fs::write(dir.join(SOURCES).join("dev/horizonlab/parity/A.java"), "class A {}").unwrap();
        fs::write(dir.join("research-lock.json"), "{}").unwrap();
    }

    #[test]
    fn only_mojang_hosts_over_https_are_fetched() {
        assert!(allowed("https://piston-meta.mojang.com/v1/packages/x/26.3.json"));
        assert!(allowed("https://resources.download.minecraft.net/ab/abcdef"));
        assert!(!allowed("http://piston-meta.mojang.com/x"));
        assert!(!allowed("https://piston-meta.mojang.com.evil.example/x"));
        assert!(!allowed("https://example.com/x"));
        assert!(fetch("https://example.com/x").is_err());
    }

    #[test]
    fn a_bundle_needs_the_lock_and_the_exporter_sources() {
        let dir = tempfile::tempdir().unwrap();
        assert!(!is_bundle(dir.path()));
        fake_bundle(dir.path());
        assert!(is_bundle(dir.path()));
    }

    #[test]
    fn syncing_copies_what_changed_and_leaves_the_rest() {
        let (from, to) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
        fake_bundle(from.path());
        sync(from.path(), to.path()).unwrap();
        let java = to.path().join(SOURCES).join("dev/horizonlab/parity/A.java");
        assert_eq!(fs::read_to_string(&java).unwrap(), "class A {}");
        fs::write(from.path().join("research-lock.json"), "{\"a\":1}").unwrap();
        sync(from.path(), to.path()).unwrap();
        assert_eq!(fs::read_to_string(to.path().join("research-lock.json")).unwrap(), "{\"a\":1}");
        assert!(!to.path().join("exporters").exists(), "no compiled exporters unless the bundle has them");
    }

    #[test]
    fn the_pins_come_from_the_lock() {
        let lock: Value = serde_json::from_str(
            r#"{"minecraft":{"version":"26.3","manifest_sha1":"m","accepted_manifest_sha1":["a","b"],"client_sha1":"c","server_sha1":"s","java_major":25}}"#,
        )
        .unwrap();
        let pins = Pins::read(&lock).unwrap();
        assert_eq!((pins.version.as_str(), pins.client_sha1.as_str(), pins.server_sha1.as_str()), ("26.3", "c", "s"));
        assert_eq!(pins.manifests, ["m", "a", "b"]);
        assert!(Pins::read(&serde_json::json!({})).is_none());
    }
}
