use crate::auth::Account;
use crate::settings::Settings;
use serde_json::json;
use std::io::Write;
use std::process::{Child, Command, Stdio};

pub fn session_json(account: &Account, client_id: &str) -> String {
    let mut session = json!({
        "name": account.name,
        "uuid": account.uuid,
        "access_token": account.access_token,
        "user_type": "msa",
    });
    if !account.refresh_token.is_empty() && !client_id.is_empty() {
        session["client_id"] = client_id.into();
        session["refresh_token"] = account.refresh_token.clone().into();
        session["expires_at"] = account.expires_at.into();
    }
    session.to_string()
}

pub fn arguments(settings: &Settings, signed_in: bool, join: bool) -> Vec<String> {
    let mut args = vec![
        "--resources".to_owned(),
        settings.resources.clone(),
        "--protocol".to_owned(),
        crate::versions::protocol_arg(settings.active_version()),
    ];
    if join && !settings.active_server().is_empty() {
        args.push("--server".into());
        args.push(settings.active_server().to_owned());
    }
    if signed_in {
        args.push("--session-stdin".into());
    } else {
        args.push("--name".into());
        args.push(settings.offline_name.clone());
    }
    args
}

pub fn launch(
    settings: &Settings,
    account: Option<&Account>,
    join: bool,
    log: Option<std::fs::File>,
) -> Result<Child, String> {
    let mut command = Command::new(&settings.client_binary);
    command.args(arguments(settings, account.is_some(), join));
    command.stdin(if account.is_some() {
        Stdio::piped()
    } else {
        Stdio::null()
    });
    if let Some(file) = log {
        let err = file.try_clone().map_err(|e| e.to_string())?;
        command.stdout(file).stderr(err);
    }
    let mut child = command
        .spawn()
        .map_err(|e| format!("Could not start {}: {e}", settings.client_binary))?;
    if let Some(account) = account {
        let mut stdin = child.stdin.take().ok_or("The game has no standard input")?;
        stdin
            .write_all(session_json(account, settings.client_id.trim()).as_bytes())
            .map_err(|e| format!("Could not hand over the session: {e}"))?;
    }
    Ok(child)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn account() -> Account {
        Account {
            name: "Alex".into(),
            uuid: "069a79f444e94726a5befca90e38aaf5".into(),
            access_token: "tok".into(),
            refresh_token: "ref".into(),
            expires_at: 1_900_000_000,
        }
    }

    #[test]
    fn the_session_json_matches_the_launch_api() {
        let value: serde_json::Value =
            serde_json::from_str(&session_json(&account(), "")).unwrap();
        assert_eq!(value["name"], "Alex");
        assert_eq!(value["access_token"], "tok");
        assert_eq!(value["user_type"], "msa");
        assert!(!session_json(&account(), "").contains("ref"));
    }

    #[test]
    fn the_game_gets_what_it_needs_to_renew_the_account_itself() {
        let value: serde_json::Value =
            serde_json::from_str(&session_json(&account(), "cid")).unwrap();
        assert_eq!(value["client_id"], "cid");
        assert_eq!(value["refresh_token"], "ref");
        assert_eq!(value["expires_at"], 1_900_000_000u64);
        let mut prism = account();
        prism.refresh_token.clear();
        assert!(!session_json(&prism, "cid").contains("refresh_token"));
    }

    #[test]
    fn signed_in_starts_read_the_session_from_stdin_and_never_carry_tokens_in_argv() {
        let settings = Settings::default();
        let args = arguments(&settings, true, true);
        assert!(args.contains(&"--session-stdin".to_owned()));
        assert!(!args.contains(&"--name".to_owned()));
        assert!(!args.iter().any(|a| a.contains("tok")));
        let offline = arguments(&settings, false, true);
        assert!(offline.contains(&"--name".to_owned()));
        assert!(!offline.contains(&"--session-stdin".to_owned()));
    }

    #[test]
    fn starting_without_joining_leaves_the_server_out() {
        let settings = Settings::default();
        let joined = arguments(&settings, false, true);
        assert!(joined.contains(&"--server".to_owned()));
        let menu = arguments(&settings, false, false);
        assert!(!menu.contains(&"--server".to_owned()));
        assert!(menu.contains(&"--protocol".to_owned()));
    }

    #[test]
    fn the_selected_servers_version_picks_the_protocol() {
        let mut settings = Settings::default();
        let protocol = |s: &Settings| {
            let args = arguments(s, false, true);
            let i = args.iter().position(|a| a == "--protocol").unwrap();
            args[i + 1].clone()
        };
        assert_eq!(protocol(&settings), "auto");
        settings.servers[0].version = "26.2".into();
        assert_eq!(protocol(&settings), "776");
        settings.servers[0].version = "26.3".into();
        assert_eq!(protocol(&settings), "native");
    }

    #[test]
    fn a_missing_game_binary_is_reported() {
        let mut settings = Settings::default();
        settings.client_binary = "/nonexistent/horizon-client".into();
        assert!(
            launch(&settings, None, true, None)
                .unwrap_err()
                .contains("Could not start")
        );
    }
}
