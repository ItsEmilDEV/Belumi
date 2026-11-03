use std::io::Write;
use std::process::{Command, Stdio};

const SERVICE: &str = "belumi-launcher";

fn attributes(account: &str) -> Vec<String> {
    vec![
        "service".into(),
        SERVICE.into(),
        "account".into(),
        account.into(),
    ]
}

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

pub fn clear(account: &str) {
    let _ = Command::new("secret-tool")
        .arg("clear")
        .args(attributes(account))
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
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
