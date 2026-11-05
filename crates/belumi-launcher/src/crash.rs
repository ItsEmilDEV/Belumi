use std::path::{Path, PathBuf};

/// What the launcher shows after the game ended badly.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Report {
    pub code: Option<i32>,
    pub report: Option<PathBuf>,
    pub last_line: String,
}

/// The crash report the client names in its log (`crash report saved to PATH`).
pub fn find_report(log: &str) -> Option<PathBuf> {
    log.lines().rev().find_map(|line| {
        let (_, path) = line.split_once("crash report saved to ")?;
        let path = path.trim();
        (!path.is_empty()).then(|| PathBuf::from(path))
    })
}

/// The last log line that says something, for a one-line explanation.
pub fn last_line(log: &str) -> String {
    log.lines()
        .rev()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .unwrap_or("")
        .chars()
        .take(220)
        .collect()
}

/// Whether an exit status means the game did not end normally (a refused command line is 2, a crash 1 or 101).
pub fn is_crash(code: Option<i32>) -> bool {
    !matches!(code, Some(0))
}

pub fn from_log(code: Option<i32>, log: &str) -> Option<Report> {
    is_crash(code).then(|| Report {
        code,
        report: find_report(log),
        last_line: last_line(log),
    })
}

/// A ZIP of the logs for support: the game log and the crash report, no settings and no tokens.
pub fn pack_for_support(log: &Path, report: Option<&Path>, out: &Path) -> std::io::Result<()> {
    let mut parts: Vec<(String, Vec<u8>)> = Vec::new();
    if let Ok(bytes) = std::fs::read(log) {
        parts.push(("game.log".into(), redact(bytes)));
    }
    if let Some(path) = report
        && let Ok(bytes) = std::fs::read(path)
    {
        parts.push(("crash-report.txt".into(), redact(bytes)));
    }
    let file = std::fs::File::create(out)?;
    let mut zip = Zip::new(file);
    for (name, bytes) in parts {
        zip.add(&name, &bytes)?;
    }
    zip.finish()
}

/// Drop what could identify the player: the home folder in paths, the user name as a word, token-shaped
/// strings and the values of token keys.
pub(crate) fn redact(bytes: Vec<u8>) -> Vec<u8> {
    let mut text = String::from_utf8_lossy(&bytes).into_owned();
    if let Some(home) = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")) {
        let home = home.to_string_lossy().into_owned();
        if home.len() > 3 {
            text = text.replace(&home, "~").replace(&home.replace('\\', "\\\\"), "~");
        }
    }
    for var in ["USER", "USERNAME", "LOGNAME"] {
        if let Ok(user) = std::env::var(var)
            && user.len() >= 4
        {
            text = replace_word(&text, &user, "user");
        }
    }
    redact_tokens(&text).into_bytes()
}

fn is_word(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

fn replace_word(text: &str, word: &str, with: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find(word) {
        let before = rest[..at].chars().next_back();
        let after = rest[at + word.len()..].chars().next();
        out.push_str(&rest[..at]);
        if before.is_some_and(is_word) || after.is_some_and(is_word) {
            out.push_str(word);
        } else {
            out.push_str(with);
        }
        rest = &rest[at + word.len()..];
    }
    out.push_str(rest);
    out
}

/// Runs of 40 or more token characters, wherever they sit in a line (inside JSON, after `=`, ...).
fn redact_tokens(text: &str) -> String {
    let token_char = |c: char| c.is_ascii_alphanumeric() || "._~+/=!*$-".contains(c);
    let mut out = String::with_capacity(text.len());
    let mut run = String::new();
    for c in text.chars() {
        if token_char(c) {
            run.push(c);
        } else {
            flush_run(&mut out, &mut run);
            out.push(c);
        }
    }
    flush_run(&mut out, &mut run);
    out
}

fn flush_run(out: &mut String, run: &mut String) {
    let letters = run.chars().filter(|c| c.is_ascii_alphanumeric()).count();
    if run.len() >= 40 && letters >= 30 && !run.starts_with("/") && !run.contains("//") {
        out.push_str("<redacted>");
    } else {
        out.push_str(run);
    }
    run.clear();
}

/// A ZIP of a folder (its files and subfolders, stored; symlinks are skipped and a file keeps its last 16 MB).
pub fn pack_dir(dir: &Path, out: &Path) -> std::io::Result<()> {
    const KEEP: usize = 16 << 20;
    fn walk(base: &Path, at: &Path, zip: &mut Zip<std::fs::File>) -> std::io::Result<()> {
        let mut entries: Vec<_> = std::fs::read_dir(at)?.flatten().collect();
        entries.sort_by_key(|e| e.file_name());
        for entry in entries {
            let path = entry.path();
            let Ok(meta) = std::fs::symlink_metadata(&path) else { continue };
            if meta.is_dir() {
                walk(base, &path, zip)?;
            } else if meta.is_file() && path != base.join("belumi-report.zip") {
                let name = path.strip_prefix(base).unwrap_or(&path).to_string_lossy().replace('\\', "/");
                let mut data = std::fs::read(&path)?;
                if data.len() > KEEP {
                    data.drain(..data.len() - KEEP);
                }
                zip.add(&name, &data)?;
            }
        }
        Ok(())
    }
    let mut zip = Zip::new(std::fs::File::create(out)?);
    walk(dir, dir, &mut zip)?;
    zip.finish()
}

/// A minimal stored (uncompressed) ZIP writer: enough for a few log files.
struct Zip<W: std::io::Write> {
    out: W,
    entries: Vec<(String, u32, u32, u32)>,
    offset: u32,
}

fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &byte in data {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            crc = if crc & 1 != 0 { (crc >> 1) ^ 0xEDB8_8320 } else { crc >> 1 };
        }
    }
    !crc
}

impl<W: std::io::Write> Zip<W> {
    fn new(out: W) -> Self {
        Self { out, entries: Vec::new(), offset: 0 }
    }

    fn add(&mut self, name: &str, data: &[u8]) -> std::io::Result<()> {
        let crc = crc32(data);
        let mut header = Vec::new();
        header.extend_from_slice(&0x0403_4b50u32.to_le_bytes());
        header.extend_from_slice(&20u16.to_le_bytes());
        header.extend_from_slice(&0u16.to_le_bytes());
        header.extend_from_slice(&0u16.to_le_bytes());
        header.extend_from_slice(&0u16.to_le_bytes());
        header.extend_from_slice(&0x21u16.to_le_bytes());
        header.extend_from_slice(&crc.to_le_bytes());
        header.extend_from_slice(&(data.len() as u32).to_le_bytes());
        header.extend_from_slice(&(data.len() as u32).to_le_bytes());
        header.extend_from_slice(&(name.len() as u16).to_le_bytes());
        header.extend_from_slice(&0u16.to_le_bytes());
        header.extend_from_slice(name.as_bytes());
        self.out.write_all(&header)?;
        self.out.write_all(data)?;
        self.entries.push((name.to_owned(), crc, data.len() as u32, self.offset));
        self.offset += (header.len() + data.len()) as u32;
        Ok(())
    }

    fn finish(mut self) -> std::io::Result<()> {
        let start = self.offset;
        let mut size = 0u32;
        for (name, crc, len, offset) in &self.entries {
            let mut c = Vec::new();
            c.extend_from_slice(&0x0201_4b50u32.to_le_bytes());
            c.extend_from_slice(&20u16.to_le_bytes());
            c.extend_from_slice(&20u16.to_le_bytes());
            c.extend_from_slice(&0u16.to_le_bytes());
            c.extend_from_slice(&0u16.to_le_bytes());
            c.extend_from_slice(&0u16.to_le_bytes());
            c.extend_from_slice(&0x21u16.to_le_bytes());
            c.extend_from_slice(&crc.to_le_bytes());
            c.extend_from_slice(&len.to_le_bytes());
            c.extend_from_slice(&len.to_le_bytes());
            c.extend_from_slice(&(name.len() as u16).to_le_bytes());
            c.extend_from_slice(&[0u8; 12]);
            c.extend_from_slice(&offset.to_le_bytes());
            c.extend_from_slice(name.as_bytes());
            size += c.len() as u32;
            self.out.write_all(&c)?;
        }
        let mut end = Vec::new();
        end.extend_from_slice(&0x0605_4b50u32.to_le_bytes());
        end.extend_from_slice(&[0u8; 4]);
        end.extend_from_slice(&(self.entries.len() as u16).to_le_bytes());
        end.extend_from_slice(&(self.entries.len() as u16).to_le_bytes());
        end.extend_from_slice(&size.to_le_bytes());
        end.extend_from_slice(&start.to_le_bytes());
        end.extend_from_slice(&0u16.to_le_bytes());
        self.out.write_all(&end)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_crash_report_is_named_in_the_log() {
        let log = "Horizon: starting\nHorizon: crash report saved to /home/a/crash-reports/crash-1.txt\n";
        assert_eq!(find_report(log), Some(PathBuf::from("/home/a/crash-reports/crash-1.txt")));
        assert_eq!(find_report("all fine\n"), None);
        assert_eq!(last_line("a\nlast thing\n\n"), "last thing");
    }

    #[test]
    fn only_a_normal_exit_is_not_a_crash() {
        assert!(!is_crash(Some(0)));
        assert!(is_crash(Some(1)));
        assert!(is_crash(Some(101)));
        assert!(is_crash(None));
        assert!(from_log(Some(0), "x").is_none());
        let r = from_log(Some(1), "crash report saved to /r.txt\n").unwrap();
        assert_eq!((r.code, r.report), (Some(1), Some(PathBuf::from("/r.txt"))));
    }

    #[test]
    fn tokens_are_redacted_inside_json_and_base64_but_paths_and_words_stay() {
        let token = "eyJ0eXAiOiJKV1QiLCJhbGciOiJSUzI1NiJ9".to_owned() + &"aB3+/=".repeat(10);
        let json = format!("{{\"access_token\":\"{token}\",\"name\":\"Steve\"}} done\n");
        let cleaned = String::from_utf8(redact(json.into_bytes())).unwrap();
        assert!(cleaned.contains("<redacted>") && !cleaned.contains("eyJ0eXAi"));
        assert!(cleaned.contains("Steve") && cleaned.contains("done"));
        let path = "/usr/lib/x86_64-linux-gnu/some/very/long/library/path/that/is/over/forty/chars.so";
        assert_eq!(String::from_utf8(redact(path.as_bytes().to_vec())).unwrap(), path);
    }

    #[test]
    fn the_user_name_is_replaced_only_as_a_whole_word() {
        assert_eq!(replace_word("bob and bobby, /home/bob/x", "bob", "user"), "user and bobby, /home/user/x");
        assert_eq!(replace_word("nothing", "bob", "user"), "nothing");
    }

    #[test]
    fn the_support_zip_is_a_readable_archive() {
        let dir = std::env::temp_dir().join(format!("belumi-crash-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("game.log"), "hello log\n").unwrap();
        std::fs::write(dir.join("crash.txt"), "boom\n").unwrap();
        let out = dir.join("support.zip");
        pack_for_support(&dir.join("game.log"), Some(&dir.join("crash.txt")), &out).unwrap();
        let bytes = std::fs::read(&out).unwrap();
        assert_eq!(&bytes[..4], b"PK\x03\x04");
        let mut archive = zip_reader(&bytes);
        assert!(archive.contains(&"game.log".to_owned()) && archive.contains(&"crash-report.txt".to_owned()));
        archive.clear();
        let _ = std::fs::remove_dir_all(&dir);
    }

    fn zip_reader(bytes: &[u8]) -> Vec<String> {
        let mut names = Vec::new();
        let mut at = 0;
        while at + 30 <= bytes.len() && &bytes[at..at + 4] == b"PK\x03\x04" {
            let len = u32::from_le_bytes(bytes[at + 18..at + 22].try_into().unwrap()) as usize;
            let name_len = u16::from_le_bytes(bytes[at + 26..at + 28].try_into().unwrap()) as usize;
            names.push(String::from_utf8_lossy(&bytes[at + 30..at + 30 + name_len]).into_owned());
            at += 30 + name_len + len;
        }
        names
    }
}
