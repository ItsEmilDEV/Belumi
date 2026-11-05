#[cfg(not(windows))]
use std::io::Write;
#[cfg(not(windows))]
use std::process::{Command, Stdio};

const SERVICE: &str = "belumi-launcher";

#[cfg(any(test, not(windows)))]
fn attributes(account: &str) -> Vec<String> {
    vec![
        "service".into(),
        SERVICE.into(),
        "account".into(),
        account.into(),
    ]
}

#[cfg(not(windows))]
pub fn store(account: &str, token: &str) -> bool {
    let child = Command::new("secret-tool")
        .args(["store", "--label=Belumi sign-in"])
        .args(attributes(account))
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
    let Ok(mut child) = child else { return false };
    if let Some(mut stdin) = child.stdin.take() {
        if stdin.write_all(token.as_bytes()).is_err() {
            return false;
        }
    }
    child.wait().is_ok_and(|status| status.success())
}

#[cfg(not(windows))]
pub fn lookup(account: &str) -> Option<String> {
    let out = Command::new("secret-tool")
        .arg("lookup")
        .args(attributes(account))
        .stderr(Stdio::null())
        .output()
        .ok()?;
    let token = String::from_utf8(out.stdout).ok()?;
    (out.status.success() && !token.trim().is_empty()).then(|| token.trim().to_owned())
}

#[cfg(not(windows))]
pub fn clear(account: &str) {
    let _ = Command::new("secret-tool")
        .arg("clear")
        .args(attributes(account))
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}

/// The Windows Credential Manager; a token the store refuses (it is limited in size) falls back to the owner-only file.
#[cfg(windows)]
pub fn store(account: &str, token: &str) -> bool {
    keyring::Entry::new(SERVICE, account)
        .and_then(|e| e.set_password(token))
        .is_ok()
}

#[cfg(windows)]
pub fn lookup(account: &str) -> Option<String> {
    let token = keyring::Entry::new(SERVICE, account).ok()?.get_password().ok()?;
    (!token.trim().is_empty()).then(|| token.trim().to_owned())
}

#[cfg(windows)]
pub fn clear(account: &str) {
    if let Ok(entry) = keyring::Entry::new(SERVICE, account) {
        let _ = entry.delete_credential();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_lookup_key_is_service_and_account_only() {
        assert_eq!(
            attributes("abc"),
            ["service", "belumi-launcher", "account", "abc"]
        );
    }
}
