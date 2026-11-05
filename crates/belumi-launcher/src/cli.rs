use crate::auth;
use crate::game::{self, Target};
use crate::settings::{SavedAccount, Settings};
use crate::{prism, secret};
use belumi_core::paths::BelumiPaths;

pub const USAGE: &str = "Usage: belumi-launcher --play [--offline] [--server HOST[:PORT] | --world NAME | --main-menu]\n\
       belumi-launcher --diagnose [--offline] [--server HOST[:PORT]] [--out FOLDER]   (benchmarks, screenshots, logs into a report folder)\n\
Starts the game without opening the launcher window, with the saved account (renewed first).\n\
  --offline     play with the offline name instead of an account\n\
  --server X    join this address (default: the server selected in the launcher)\n\
  --world NAME  open this singleplayer world\n\
  --main-menu   start in the main menu\n";

#[derive(Debug, PartialEq, Eq)]
pub struct Request {
    pub offline: bool,
    pub target: Target,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Command {
    Play(Request),
    Diagnose { offline: bool, server: Option<String>, out: Option<std::path::PathBuf> },
}

/// `Ok(None)` when the arguments do not ask for a headless start.
pub fn parse(args: impl IntoIterator<Item = String>) -> Result<Option<Command>, String> {
    let mut args = args.into_iter().peekable();
    if args.peek().is_none() {
        return Ok(None);
    }
    let (mut play, mut offline, mut target) = (false, false, Target::Server);
    let (mut diagnose, mut out) = (false, None);
    let mut server_arg: Option<String> = None;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--play" => play = true,
            "--diagnose" => diagnose = true,
            "--out" => out = Some(std::path::PathBuf::from(args.next().ok_or("--out needs a folder")?)),
            "--offline" => offline = true,
            "--main-menu" => target = Target::MainMenu,
            "--server" => {
                let address = args.next().ok_or("--server needs an address")?;
                server_arg = Some(address.clone());
                target = Target::Address(address);
            }
            "--world" => {
                target = Target::World(args.next().ok_or("--world needs a world name")?);
            }
            "--help" | "-h" => return Err(String::new()),
            other => return Err(format!("unknown option {other}")),
        }
    }
    if diagnose {
        return Ok(Some(Command::Diagnose { offline, server: server_arg, out }));
    }
    Ok(play.then_some(Command::Play(Request { offline, target })))
}

fn account_for(settings: &mut Settings, path: &std::path::Path) -> Result<auth::Account, String> {
    for saved in &mut settings.accounts {
        if saved.refresh_token.is_empty()
            && let Some(token) = secret::lookup(&saved.uuid)
        {
            saved.refresh_token = token;
        }
    }
    if let Some(saved) = settings.active_saved().cloned()
        && !saved.refresh_token.is_empty()
    {
        let ms = auth::refresh(settings.client_id.trim(), &saved.refresh_token)?;
        let account = auth::minecraft_account(&ms)?;
        settings.remember(SavedAccount {
            name: account.name.clone(),
            uuid: account.uuid.clone(),
            refresh_token: account.refresh_token.clone(),
        });
        let mut on_disk = settings.clone();
        for saved in &mut on_disk.accounts {
            if !saved.refresh_token.is_empty() && secret::store(&saved.uuid, &saved.refresh_token) {
                saved.refresh_token.clear();
            }
        }
        let _ = on_disk.save(path);
        return Ok(account);
    }
    if settings.use_prism {
        let p = prism::load(&settings.prism_accounts_path, crate::unix_now())?;
        return Ok(auth::Account {
            name: p.name,
            uuid: p.uuid,
            access_token: p.access_token,
            refresh_token: String::new(),
            expires_at: p.expires_at,
        });
    }
    Err("No signed-in account. Open the launcher once and sign in, or use --offline.".into())
}

/// The server a diagnose build visits: `diagnose-server.txt` beside the program.
pub fn diagnose_server() -> Option<String> {
    let exe = std::env::current_exe().ok()?;
    let text = std::fs::read_to_string(exe.parent()?.join("diagnose-server.txt")).ok()?;
    let line = text.lines().map(str::trim).find(|l| !l.is_empty() && !l.starts_with('#'))?;
    Some(line.to_owned())
}

/// Runs a headless start; `None` means "open the window".
pub fn run(args: impl IntoIterator<Item = String>) -> Option<i32> {
    let command = match parse(args) {
        Ok(Some(r)) => r,
        Ok(None) => return None,
        Err(message) => {
            if !message.is_empty() {
                eprintln!("belumi-launcher: {message}");
            }
            eprint!("{}", USAGE);
            return Some(if message.is_empty() { 0 } else { 2 });
        }
    };
    let paths = BelumiPaths::new();
    let path = paths.config_dir().join("launcher.json");
    let mut settings = Settings::load(&path);
    let request = match command {
        Command::Play(request) => request,
        Command::Diagnose { offline, server, out } => {
            let account = if offline { None } else { account_for(&mut settings, &path).ok() };
            let server = server.or_else(diagnose_server);
            let out = out.unwrap_or_else(|| paths.data_dir().join("belumi-report"));
            let _ = std::fs::remove_dir_all(&out);
            return Some(match crate::diagnose::run(&settings, account.as_ref(), server.as_deref(), &out, &|line| println!("{line}")) {
                Ok(zip) => {
                    println!("Done. Send this folder (or the ZIP inside it): {}", out.display());
                    println!("{}", zip.display());
                    0
                }
                Err(e) => {
                    eprintln!("belumi-launcher: {e}");
                    1
                }
            });
        }
    };
    let account = if request.offline {
        None
    } else {
        match account_for(&mut settings, &path) {
            Ok(a) => Some(a),
            Err(e) => {
                eprintln!("belumi-launcher: {e}");
                return Some(1);
            }
        }
    };
    let log = std::fs::File::create(paths.logs_dir().join("game.log")).ok();
    match game::launch(&settings, account.as_ref(), &request.target, log) {
        Ok(mut child) => Some(child.wait().ok().and_then(|s| s.code()).unwrap_or(1)),
        Err(e) => {
            eprintln!("belumi-launcher: {e}");
            Some(1)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| (*s).to_owned()).collect()
    }

    #[test]
    fn no_arguments_open_the_window() {
        assert_eq!(parse(args(&[])), Ok(None));
        assert_eq!(parse(args(&["--offline"])), Ok(None));
    }

    #[test]
    fn a_play_request_names_its_target() {
        assert_eq!(parse(args(&["--play"])), Ok(Some(Command::Play(Request { offline: false, target: Target::Server }))));
        assert_eq!(
            parse(args(&["--play", "--offline", "--world", "My World"])),
            Ok(Some(Command::Play(Request { offline: true, target: Target::World("My World".into()) })))
        );
        assert_eq!(
            parse(args(&["--play", "--server", "play.example.net"])),
            Ok(Some(Command::Play(Request { offline: false, target: Target::Address("play.example.net".into()) })))
        );
        assert_eq!(parse(args(&["--play", "--main-menu"])), Ok(Some(Command::Play(Request { offline: false, target: Target::MainMenu }))));
    }

    #[test]
    fn a_diagnose_request_takes_a_server_and_a_folder() {
        assert_eq!(
            parse(args(&["--diagnose", "--server", "play.example.net", "--out", "/tmp/r"])),
            Ok(Some(Command::Diagnose { offline: false, server: Some("play.example.net".into()), out: Some("/tmp/r".into()) }))
        );
        assert_eq!(
            parse(args(&["--diagnose", "--offline"])),
            Ok(Some(Command::Diagnose { offline: true, server: None, out: None }))
        );
    }

    #[test]
    fn bad_arguments_are_refused() {
        assert!(parse(args(&["--play", "--world"])).is_err());
        assert!(parse(args(&["--wat"])).unwrap_err().contains("--wat"));
        assert_eq!(parse(args(&["--help"])), Err(String::new()));
    }
}
