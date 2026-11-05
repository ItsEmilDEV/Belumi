use std::path::Path;
use std::process::Command;

/// Open a file or folder with the desktop's default program.
pub fn open(path: &Path) {
    let program = if cfg!(windows) {
        "explorer"
    } else if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };
    let _ = Command::new(program).arg(path).spawn();
}
