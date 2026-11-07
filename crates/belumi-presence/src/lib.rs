//! Discord Rich Presence over Discord's local IPC socket (Discord's RPC docs: `discord-ipc-N`, frames
//! of opcode u32 LE + length u32 LE + JSON). One thread talks to the Discord app on this computer: it
//! connects when Discord runs, retries every 15 s when it doesn't, keeps under Discord's update limit,
//! sends the activity again every minute and clears it on stop. The game and the launcher both use it;
//! each hands over the activity JSON it wants shown.
use std::collections::VecDeque;
use std::io::{self, Read, Write};
use std::path::PathBuf;
use std::sync::{Arc, Condvar, Mutex};
use std::thread::Thread;
use std::time::{Duration, Instant};

use serde_json::{Value, json};

/// Belumi's own application on Discord's developer portal (its ID is public: it names the game).
/// Rich Presence assets there: `belumi-minecraft` (in the game), `belumi` (in the launcher) and the
/// four game modes. Never put another application's ID here.
pub const APP_ID: &str = "1568528963675099208";

/// The application ID in use: `BELUMI_DISCORD_APP_ID` in the environment, else [`APP_ID`]; `None`
/// when it is empty or not digits, which switches the presence off.
pub fn app_id() -> Option<String> {
    let from_env = std::env::var("BELUMI_DISCORD_APP_ID").ok();
    let id = from_env.as_deref().unwrap_or(APP_ID).trim();
    (!id.is_empty() && id.bytes().all(|b| b.is_ascii_digit())).then(|| id.to_owned())
}

/// Whether the player switched Rich Presence off for this run (`BELUMI_NO_DISCORD=1`).
pub fn disabled_by_env() -> bool {
    ["BELUMI_NO_DISCORD", "HORIZON_NO_DISCORD"]
        .into_iter()
        .any(|k| std::env::var_os(k).is_some_and(|v| v != "0"))
}

/// Points a program at a test's fake Discord (a folder holding `discord-ipc-0`).
const IPC_DIR_ENV: &str = "BELUMI_DISCORD_IPC_DIR";
const OLD_IPC_DIR_ENV: &str = "HORIZON_DISCORD_IPC_DIR";

/// Discord drops activity updates beyond 5 in 20 seconds.
const BURST: usize = 5;
const WINDOW: Duration = Duration::from_secs(20);
/// Between two tries to reach Discord while it isn't running.
const RETRY: Duration = Duration::from_secs(15);
/// The activity is sent again this often, so a Discord restarted meanwhile gets it back.
const REFRESH: Duration = Duration::from_secs(60);

const HANDSHAKE: u32 = 0;
const FRAME: u32 = 1;
const CLOSE: u32 = 2;
const PING: u32 = 3;
const PONG: u32 = 4;

struct Shared {
    wanted: Option<Value>,
    stop: bool,
    stopped: bool,
}

struct Inner {
    app_id: String,
    shared: Mutex<Shared>,
    stopped: Condvar,
}

impl Inner {
    fn lock(&self) -> std::sync::MutexGuard<'_, Shared> {
        self.shared.lock().unwrap_or_else(|e| e.into_inner())
    }
}

/// A running presence thread.
pub struct Presence {
    inner: Arc<Inner>,
    thread: Thread,
}

impl Presence {
    /// Starts the thread for the Discord application `app_id` (digits only). `None` for a bad ID.
    /// With nothing set it stays idle: it connects to Discord only once there is an activity.
    pub fn start(app_id: &str) -> Option<Presence> {
        let app_id = app_id.trim();
        if app_id.is_empty() || !app_id.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        let inner = Arc::new(Inner {
            app_id: app_id.to_owned(),
            shared: Mutex::new(Shared { wanted: None, stop: false, stopped: false }),
            stopped: Condvar::new(),
        });
        let worker = inner.clone();
        match std::thread::Builder::new().name("Discord presence".into()).spawn(move || run(&worker)) {
            Ok(handle) => Some(Presence { inner, thread: handle.thread().clone() }),
            Err(e) => {
                log::warn!("Belumi: no Discord presence: {e}");
                None
            }
        }
    }

    /// What to show now (`None` clears it). Only a change wakes the thread.
    pub fn set(&self, activity: Option<Value>) {
        let mut shared = self.inner.lock();
        if shared.wanted != activity {
            shared.wanted = activity;
            self.thread.unpark();
        }
    }

    /// Clears the activity and ends the thread, waiting at most a moment for Discord.
    pub fn stop(&self) {
        let mut shared = self.inner.lock();
        shared.stop = true;
        self.thread.unpark();
        let _ = self.inner.stopped.wait_timeout_while(shared, Duration::from_millis(300), |s| !s.stopped);
    }
}

fn run(inner: &Inner) {
    let mut client: Option<Client> = None;
    let mut sent: Option<(Value, Instant)> = None;
    let mut last_try: Option<Instant> = None;
    let mut limit = RateLimit::default();
    loop {
        let (wanted, stop) = {
            let s = inner.lock();
            (s.wanted.clone(), s.stop)
        };
        let Some(wanted) = wanted.filter(|_| !stop) else {
            if let Some(mut c) = client.take() {
                let _ = c.set_activity(None);
                c.close();
            }
            sent = None;
            if stop {
                inner.lock().stopped = true;
                inner.stopped.notify_all();
                return;
            }
            std::thread::park();
            continue;
        };
        let now = Instant::now();
        if client.is_none() {
            if let Some(wait) = last_try.and_then(|t| RETRY.checked_sub(now - t)) {
                std::thread::park_timeout(wait);
                continue;
            }
            last_try = Some(now);
            client = Client::connect(&inner.app_id);
            if client.is_none() {
                continue;
            }
        }
        let due = sent.as_ref().is_none_or(|(w, at)| *w != wanted || now - *at >= REFRESH);
        if !due {
            let (_, at) = sent.as_ref().expect("sent");
            std::thread::park_timeout(REFRESH - (now - *at));
            continue;
        }
        let wait = limit.wait(now);
        if !wait.is_zero() {
            std::thread::park_timeout(wait);
            continue;
        }
        limit.record(now);
        let c = client.as_mut().expect("connected");
        match c.set_activity(Some(wanted.clone())) {
            Ok(()) => sent = Some((wanted, now)),
            Err(e) => {
                log::info!("Belumi: Discord presence disconnected ({e})");
                client = None;
                sent = None;
            }
        }
    }
}

/// At most [`BURST`] updates in any [`WINDOW`].
#[derive(Default)]
struct RateLimit {
    sent: VecDeque<Instant>,
}
impl RateLimit {
    /// How long until the next update may go.
    fn wait(&self, now: Instant) -> Duration {
        if self.sent.len() < BURST {
            return Duration::ZERO;
        }
        (self.sent[0] + WINDOW).saturating_duration_since(now)
    }
    fn record(&mut self, now: Instant) {
        if self.sent.len() == BURST {
            self.sent.pop_front();
        }
        self.sent.push_back(now);
    }
}

fn encode(op: u32, payload: &Value) -> Vec<u8> {
    let body = payload.to_string();
    let mut frame = Vec::with_capacity(8 + body.len());
    frame.extend_from_slice(&op.to_le_bytes());
    frame.extend_from_slice(&(body.len() as u32).to_le_bytes());
    frame.extend_from_slice(body.as_bytes());
    frame
}

fn read_frame(r: &mut impl Read) -> io::Result<(u32, Value)> {
    let mut header = [0; 8];
    r.read_exact(&mut header)?;
    let op = u32::from_le_bytes(header[..4].try_into().expect("4 bytes"));
    let len = u32::from_le_bytes(header[4..].try_into().expect("4 bytes")) as usize;
    if len > 1 << 20 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "frame too large",
        ));
    }
    let mut body = vec![0; len];
    r.read_exact(&mut body)?;
    Ok((op, serde_json::from_slice(&body)?))
}

trait Pipe: Read + Write + Send {}
impl<T: Read + Write + Send> Pipe for T {}

struct Client {
    pipe: Box<dyn Pipe>,
    nonce: u64,
}
impl Client {
    /// The first Discord socket that answers the handshake with READY.
    fn connect(app_id: &str) -> Option<Self> {
        sockets().into_iter().find_map(|path| {
            let mut client = Client {
                pipe: open(&path).ok()?,
                nonce: 0,
            };
            match client.handshake(app_id) {
                Ok(()) => {
                    log::info!("Belumi: connected to Discord for Rich Presence");
                    Some(client)
                }
                Err(e) => {
                    log::debug!("Belumi: Discord at {} refused: {e}", path.display());
                    None
                }
            }
        })
    }
    fn handshake(&mut self, app_id: &str) -> io::Result<()> {
        let hello = json!({"v": 1, "client_id": app_id});
        self.pipe.write_all(&encode(HANDSHAKE, &hello))?;
        self.reply(|frame| frame["evt"] == "READY")
    }
    fn set_activity(&mut self, activity: Option<Value>) -> io::Result<()> {
        self.nonce += 1;
        let nonce = self.nonce.to_string();
        let command = json!({
            "cmd": "SET_ACTIVITY",
            "args": {"pid": std::process::id(), "activity": activity},
            "nonce": nonce,
        });
        self.pipe.write_all(&encode(FRAME, &command))?;
        self.reply(|frame| frame["nonce"] == nonce.as_str())
    }
    /// Reads until the frame `ours` picks, answering pings; an ERROR is logged, not fatal.
    fn reply(&mut self, ours: impl Fn(&Value) -> bool) -> io::Result<()> {
        loop {
            match read_frame(&mut self.pipe)? {
                (FRAME, frame) if ours(&frame) => {
                    if frame["evt"] == "ERROR" {
                        log::info!("Belumi: Discord presence: {}", frame["data"]["message"]);
                    }
                    return Ok(());
                }
                (PING, frame) => self.pipe.write_all(&encode(PONG, &frame))?,
                (CLOSE, frame) => {
                    return Err(io::Error::new(
                        io::ErrorKind::ConnectionAborted,
                        frame["message"].as_str().unwrap_or("closed").to_owned(),
                    ));
                }
                _ => {}
            }
        }
    }
    fn close(mut self) {
        let _ = self.pipe.write_all(&encode(CLOSE, &json!({})));
    }
}

/// Where Discord listens: `discord-ipc-0..9` in the temporary folders (and Flatpak's and
/// Snap's subfolders) on Linux and macOS, named pipes on Windows.
fn sockets() -> Vec<PathBuf> {
    sockets_with(|key| std::env::var_os(key), cfg!(windows))
}

fn sockets_with(var: impl Fn(&str) -> Option<std::ffi::OsString>, windows: bool) -> Vec<PathBuf> {
    let names = (0..10).map(|n| format!("discord-ipc-{n}"));
    if let Some(dir) = var(IPC_DIR_ENV).or_else(|| var(OLD_IPC_DIR_ENV)) {
        return names.map(|n| PathBuf::from(&dir).join(n)).collect();
    }
    if windows {
        return names.map(|n| PathBuf::from(r"\\?\pipe\").join(n)).collect();
    }
    let mut dirs: Vec<PathBuf> = ["XDG_RUNTIME_DIR", "TMPDIR", "TMP", "TEMP"]
        .into_iter()
        .filter_map(&var)
        .map(PathBuf::from)
        .collect();
    dirs.push("/tmp".into());
    dirs.dedup();
    let mut paths = Vec::new();
    for dir in dirs {
        for sub in ["", "app/com.discordapp.Discord", "snap.discord"] {
            paths.extend(names.clone().map(|n| dir.join(sub).join(n)));
        }
    }
    paths
}

#[cfg(unix)]
fn open(path: &std::path::Path) -> io::Result<Box<dyn Pipe>> {
    let stream = std::os::unix::net::UnixStream::connect(path)?;
    // A Discord that stops answering costs this thread a few seconds, never the game.
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    Ok(Box::new(stream))
}

#[cfg(windows)]
fn open(path: &std::path::Path) -> io::Result<Box<dyn Pipe>> {
    let pipe = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)?;
    Ok(Box::new(pipe))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The folders a real Discord listens in: `$TMPDIR` on macOS, `$XDG_RUNTIME_DIR` on Linux (and its
    /// Flatpak and Snap subfolders), named pipes on Windows.
    #[test]
    fn sockets_cover_where_discord_listens() {
        let env = |pairs: &'static [(&'static str, &'static str)]| {
            move |key: &str| {
                pairs
                    .iter()
                    .find(|(k, _)| *k == key)
                    .map(|(_, v)| std::ffi::OsString::from(v))
            }
        };
        let macos = sockets_with(env(&[("TMPDIR", "/var/folders/xy/abc/T/")]), false);
        assert_eq!(
            macos[0],
            PathBuf::from("/var/folders/xy/abc/T/discord-ipc-0")
        );
        assert!(macos.contains(&PathBuf::from("/var/folders/xy/abc/T/discord-ipc-9")));
        assert!(macos.contains(&PathBuf::from("/tmp/discord-ipc-0")));

        let linux = sockets_with(
            env(&[("XDG_RUNTIME_DIR", "/run/user/1000"), ("TMPDIR", "/tmp")]),
            false,
        );
        assert_eq!(linux[0], PathBuf::from("/run/user/1000/discord-ipc-0"));
        for path in [
            "/run/user/1000/app/com.discordapp.Discord/discord-ipc-0",
            "/run/user/1000/snap.discord/discord-ipc-0",
            "/tmp/discord-ipc-0",
        ] {
            assert!(linux.contains(&PathBuf::from(path)), "{path}");
        }
        assert_eq!(linux.len(), 2 * 3 * 10, "/tmp is listed once");

        let bare = sockets_with(env(&[]), false);
        assert_eq!(bare[0], PathBuf::from("/tmp/discord-ipc-0"));

        let windows = sockets_with(env(&[("TEMP", r"C:\Temp")]), true);
        assert_eq!(windows.len(), 10);
        // `Path::join` only uses `\` as the separator on Windows itself.
        #[cfg(windows)]
        assert_eq!(windows[0], PathBuf::from(r"\\?\pipe\discord-ipc-0"));

        let fake = sockets_with(
            env(&[("BELUMI_DISCORD_IPC_DIR", "/tmp/hz"), ("TMPDIR", "/x")]),
            false,
        );
        assert_eq!(fake.len(), 10);
        assert_eq!(fake[0], PathBuf::from("/tmp/hz/discord-ipc-0"));
    }

    #[test]
    fn frames_round_trip() {
        let frame = encode(FRAME, &json!({"cmd": "SET_ACTIVITY"}));
        assert_eq!(&frame[..4], &[1, 0, 0, 0]);
        assert_eq!(frame[4..8], (frame.len() as u32 - 8).to_le_bytes());
        let (op, value) = read_frame(&mut frame.as_slice()).unwrap();
        assert_eq!((op, value), (FRAME, json!({"cmd": "SET_ACTIVITY"})));
        assert!(read_frame(&mut &frame[..6]).is_err());
        let huge = [1, 0, 0, 0, 0, 0, 0, 2];
        assert!(read_frame(&mut huge.as_slice()).is_err());
    }

    #[test]
    fn at_most_five_updates_in_twenty_seconds() {
        let t = Instant::now();
        let mut limit = RateLimit::default();
        for i in 0..5 {
            let now = t + Duration::from_secs(i);
            assert_eq!(limit.wait(now), Duration::ZERO);
            limit.record(now);
        }
        assert_eq!(
            limit.wait(t + Duration::from_secs(5)),
            Duration::from_secs(15)
        );
        assert_eq!(limit.wait(t + WINDOW), Duration::ZERO);
        limit.record(t + WINDOW);
        assert_eq!(limit.wait(t + WINDOW), Duration::from_secs(1));
    }

    /// A fake Discord: answers the handshake with READY, each command with its nonce, after a ping.
    #[cfg(unix)]
    #[test]
    fn talks_to_discord_over_its_socket() {
        let dir = std::env::temp_dir().join(format!("belumi-presence-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("discord-ipc-0");
        let _ = std::fs::remove_file(&path);
        let listener = std::os::unix::net::UnixListener::bind(&path).unwrap();
        let server = std::thread::spawn(move || {
            let (mut s, _) = listener.accept().unwrap();
            let (op, hello) = read_frame(&mut s).unwrap();
            assert_eq!((op, &hello["v"]), (HANDSHAKE, &json!(1)));
            let ready = json!({"cmd": "DISPATCH", "evt": "READY", "data": {"v": 1}});
            s.write_all(&encode(FRAME, &ready)).unwrap();
            let mut got = Vec::new();
            for _ in 0..2 {
                let (op, cmd) = read_frame(&mut s).unwrap();
                assert_eq!(op, FRAME);
                s.write_all(&encode(PING, &json!({"n": 1}))).unwrap();
                assert_eq!(read_frame(&mut s).unwrap(), (PONG, json!({"n": 1})));
                let reply = json!({"cmd": "SET_ACTIVITY", "nonce": cmd["nonce"], "data": null});
                s.write_all(&encode(FRAME, &reply)).unwrap();
                got.push(cmd);
            }
            got
        });
        let mut client = Client {
            pipe: open(&path).unwrap(),
            nonce: 0,
        };
        client.handshake("1").unwrap();
        let shown = Some(json!({"details": "Singleplayer", "state": "In the End"}));
        client.set_activity(shown.clone()).unwrap();
        client.set_activity(None).unwrap();
        let got = server.join().unwrap();
        assert_eq!(got[0]["cmd"], "SET_ACTIVITY");
        assert_eq!(got[0]["args"]["pid"], std::process::id());
        assert_eq!(got[0]["args"]["activity"], shown.unwrap());
        assert_eq!(got[1]["args"]["activity"], Value::Null);
        assert_ne!(got[0]["nonce"], got[1]["nonce"]);
        let _ = std::fs::remove_dir_all(dir);
    }
}
