use crate::settings::Settings;
use std::path::Path;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Check {
    pub what: &'static str,
    pub ok: bool,
    pub hint: String,
}

fn writable(dir: &Path) -> bool {
    if std::fs::create_dir_all(dir).is_err() {
        return false;
    }
    let probe = dir.join(".belumi-write-test");
    let ok = std::fs::write(&probe, b"x").is_ok();
    let _ = std::fs::remove_file(probe);
    ok
}

fn vulkan_available() -> bool {
    if cfg!(windows) {
        return true;
    }
    [
        "/usr/lib/libvulkan.so.1",
        "/usr/lib64/libvulkan.so.1",
        "/usr/lib/x86_64-linux-gnu/libvulkan.so.1",
        "/usr/lib/aarch64-linux-gnu/libvulkan.so.1",
        "/lib/x86_64-linux-gnu/libvulkan.so.1",
    ]
    .iter()
    .any(|p| Path::new(p).exists())
}

/// The first-start checks: the game file, the prepared resources, a writable game folder, a graphics driver.
pub fn run(settings: &Settings) -> Vec<Check> {
    let binary = Path::new(settings.client_binary.trim());
    let resources = Path::new(settings.resources.trim());
    let game = crate::worlds::game_dir(&settings.game_dir);
    vec![
        Check {
            what: "Game file",
            ok: binary.is_file(),
            hint: format!("{} was not found. Set it under Settings.", binary.display()),
        },
        Check {
            what: "Resources",
            ok: resources.join("horizon-resources.json").is_file(),
            hint: format!(
                "No prepared resources in {}. Prepare them with horizon-prepare or set the folder under Settings.",
                resources.display()
            ),
        },
        Check {
            what: "Game folder",
            ok: writable(&game),
            hint: format!("The game cannot write to {}.", game.display()),
        },
        Check {
            what: "Graphics driver",
            ok: vulkan_available(),
            hint: "No Vulkan driver found. Install your GPU's Vulkan driver (vulkan-icd-loader and mesa or the vendor driver).".to_owned(),
        },
    ]
}

pub fn failed(checks: &[Check]) -> Vec<&Check> {
    checks.iter().filter(|c| !c.ok).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_missing_game_file_and_resources_are_reported_with_their_paths() {
        let dir = std::env::temp_dir().join(format!("belumi-check-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let mut settings = Settings::default();
        settings.client_binary = dir.join("nope").display().to_string();
        settings.resources = dir.join("res").display().to_string();
        settings.game_dir = dir.join("game").display().to_string();
        let checks = run(&settings);
        let bad = failed(&checks);
        assert!(bad.iter().any(|c| c.what == "Game file" && c.hint.contains("nope")));
        assert!(bad.iter().any(|c| c.what == "Resources"));
        assert!(checks.iter().any(|c| c.what == "Game folder" && c.ok));
        std::fs::write(dir.join("exe"), b"x").unwrap();
        std::fs::create_dir_all(dir.join("res")).unwrap();
        std::fs::write(dir.join("res/horizon-resources.json"), b"{}").unwrap();
        settings.client_binary = dir.join("exe").display().to_string();
        let checks = run(&settings);
        assert!(!failed(&checks).iter().any(|c| c.what == "Game file" || c.what == "Resources"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
