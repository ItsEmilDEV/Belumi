use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct World {
    pub folder: String,
    pub played: Option<SystemTime>,
}

/// The worlds in `<game>/saves`: folders with a `level.dat`, newest first.
pub fn scan(game_dir: &Path) -> Vec<World> {
    let mut worlds: Vec<World> = std::fs::read_dir(game_dir.join("saves"))
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|entry| {
            let level = entry.path().join("level.dat");
            let meta = std::fs::metadata(&level).ok()?;
            Some(World {
                folder: entry.file_name().to_string_lossy().into_owned(),
                played: meta.modified().ok(),
            })
        })
        .collect();
    worlds.sort_by(|a, b| b.played.cmp(&a.played).then_with(|| a.folder.cmp(&b.folder)));
    worlds
}

/// "just now", "5 min ago", "3 h ago", "2 days ago".
pub fn ago(played: SystemTime, now: SystemTime) -> String {
    let secs = now.duration_since(played).unwrap_or(Duration::ZERO).as_secs();
    match secs {
        0..=59 => "just now".to_owned(),
        60..=3599 => format!("{} min ago", secs / 60),
        3600..=86_399 => format!("{} h ago", secs / 3600),
        _ => format!("{} days ago", secs / 86_400),
    }
}

pub fn game_dir(setting: &str) -> PathBuf {
    let trimmed = setting.trim();
    if trimmed.is_empty() {
        std::env::current_dir().unwrap_or_default()
    } else {
        PathBuf::from(trimmed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn worlds_are_the_saves_with_a_level_file_newest_first() {
        let dir = std::env::temp_dir().join(format!("belumi-worlds-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        for name in ["Old", "New", "Broken"] {
            std::fs::create_dir_all(dir.join("saves").join(name)).unwrap();
        }
        std::fs::write(dir.join("saves/Old/level.dat"), b"x").unwrap();
        std::thread::sleep(Duration::from_millis(30));
        std::fs::write(dir.join("saves/New/level.dat"), b"x").unwrap();
        let worlds = scan(&dir);
        assert_eq!(worlds.iter().map(|w| w.folder.as_str()).collect::<Vec<_>>(), ["New", "Old"]);
        assert!(scan(&dir.join("nowhere")).is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn times_read_naturally() {
        let now = SystemTime::now();
        assert_eq!(ago(now, now), "just now");
        assert_eq!(ago(now - Duration::from_secs(300), now), "5 min ago");
        assert_eq!(ago(now - Duration::from_secs(7200), now), "2 h ago");
        assert_eq!(ago(now - Duration::from_secs(3 * 86_400), now), "3 days ago");
    }
}
