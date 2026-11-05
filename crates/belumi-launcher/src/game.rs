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

/// What the game opens: the selected server, a given address, a world, or its main menu.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Target {
    Server,
    Address(String),
    World(String),
    MainMenu,
}

pub fn arguments(settings: &Settings, signed_in: bool, target: &Target) -> Vec<String> {
    let mut args = vec![
        "--resources".to_owned(),
        settings.resources.clone(),
        "--protocol".to_owned(),
        crate::versions::protocol_arg(settings.active_version()),
    ];
    match target {
        Target::Server if !settings.active_server().is_empty() => {
            args.push("--server".into());
            args.push(settings.active_server().to_owned());
        }
        Target::Address(address) if !address.trim().is_empty() => {
            args.push("--server".into());
            args.push(address.trim().to_owned());
        }
        Target::World(name) if !name.trim().is_empty() => {
            args.push("--quickPlaySingleplayer".into());
            args.push(name.trim().to_owned());
        }
        _ => {}
    }
    if !settings.game_dir.trim().is_empty() {
        args.push("--game-dir".into());
        args.push(settings.game_dir.trim().to_owned());
    }
    if settings.fullscreen {
        args.push("--fullscreen".into());
    } else if settings.window_width >= 320 && settings.window_height >= 240 {
        args.push("--width".into());
        args.push(settings.window_width.to_string());
        args.push("--height".into());
        args.push(settings.window_height.to_string());
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
    target: &Target,
    log: Option<std::fs::File>,
) -> Result<Child, String> {
    launch_with(settings, account, target, log, &[], &[])
}

/// `launch` with more arguments and environment variables (the diagnose run's).
pub fn launch_with(
    settings: &Settings,
    account: Option<&Account>,
    target: &Target,
    log: Option<std::fs::File>,
    extra_args: &[String],
    extra_env: &[(&str, &str)],
) -> Result<Child, String> {
    let mut command = Command::new(&settings.client_binary);
    command.args(arguments(settings, account.is_some(), target));
    command.args(extra_args);
    command.envs(extra_env.iter().copied());
    match settings.graphics_api.as_str() {
        "vulkan" => {
            command.env("WGPU_BACKEND", "vulkan");
        }
        "dx12" => {
            command.env("WGPU_BACKEND", "dx12");
        }
        _ => {}
    }
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
        let args = arguments(&settings, true, &Target::Server);
        assert!(args.contains(&"--session-stdin".to_owned()));
        assert!(!args.contains(&"--name".to_owned()));
        assert!(!args.iter().any(|a| a.contains("tok")));
        let offline = arguments(&settings, false, &Target::Server);
        assert!(offline.contains(&"--name".to_owned()));
        assert!(!offline.contains(&"--session-stdin".to_owned()));
    }

    #[test]
    fn starting_without_joining_leaves_the_server_out() {
        let settings = Settings::default();
        let joined = arguments(&settings, false, &Target::Server);
        assert!(joined.contains(&"--server".to_owned()));
        let menu = arguments(&settings, false, &Target::MainMenu);
        assert!(!menu.contains(&"--server".to_owned()));
        assert!(menu.contains(&"--protocol".to_owned()));
    }

    #[test]
    fn a_world_a_given_address_and_the_launch_options_become_arguments() {
        let mut settings = Settings::default();
        let world = arguments(&settings, false, &Target::World("My World".into()));
        let at = world.iter().position(|a| a == "--quickPlaySingleplayer").unwrap();
        assert_eq!(world[at + 1], "My World");
        assert!(!world.contains(&"--server".to_owned()));
        let address = arguments(&settings, false, &Target::Address(" play.example.net ".into()));
        let at = address.iter().position(|a| a == "--server").unwrap();
        assert_eq!(address[at + 1], "play.example.net");
        settings.fullscreen = true;
        settings.game_dir = "/games/belumi".into();
        let full = arguments(&settings, false, &Target::MainMenu);
        assert!(full.contains(&"--fullscreen".to_owned()));
        assert!(!full.contains(&"--width".to_owned()));
        let at = full.iter().position(|a| a == "--game-dir").unwrap();
        assert_eq!(full[at + 1], "/games/belumi");
        settings.fullscreen = false;
        settings.window_width = 1600;
        settings.window_height = 900;
        let sized = arguments(&settings, false, &Target::MainMenu);
        let at = sized.iter().position(|a| a == "--width").unwrap();
        assert_eq!(&sized[at..at + 4], ["--width", "1600", "--height", "900"]);
        settings.window_width = 100;
        assert!(!arguments(&settings, false, &Target::MainMenu).contains(&"--width".to_owned()));
    }

    #[test]
    fn the_selected_servers_version_picks_the_protocol() {
        let mut settings = Settings::default();
        let protocol = |s: &Settings| {
            let args = arguments(s, false, &Target::Server);
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
            launch(&settings, None, &Target::Server, None)
                .unwrap_err()
                .contains("Could not start")
        );
    }
}
