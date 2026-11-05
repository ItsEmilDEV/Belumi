#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod auth;
mod cli;
mod check;
mod crash;
mod diagnose;
mod fonts;
mod game;
mod heads;
mod icons;
mod i18n;
mod kit;
mod os;
mod prism;
mod secret;
mod settings;
mod worlds;
mod status;
mod theme;
mod ui;
mod update;
mod versions;

use auth::{Account, DeviceCode};
use belumi_core::paths::BelumiPaths;
use eframe::egui;
use settings::{SavedAccount, Settings};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// A sender that also wakes the window, so an event shows at once instead of at the next tick.
#[derive(Clone)]
struct Notifier {
    tx: Sender<Event>,
    ctx: Arc<std::sync::Mutex<Option<egui::Context>>>,
}

impl Notifier {
    fn send(&self, event: Event) -> Result<(), std::sync::mpsc::SendError<Event>> {
        let sent = self.tx.send(event);
        if let Ok(ctx) = self.ctx.lock()
            && let Some(ctx) = ctx.as_ref()
        {
            ctx.request_repaint();
        }
        sent
    }
}

enum Event {
    Status(String),
    Code(DeviceCode),
    SignedIn(Account),
    Failed(String),
    Started,
    Exited(String, Option<i32>),
    Probed(String, Option<status::ServerStatus>),
    Diagnosed(Result<std::path::PathBuf, String>),
    Update(Option<update::Release>),
    Head(String, Option<Vec<u8>>),
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Source {
    Own,
    Prism,
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

struct App {
    settings: Settings,
    settings_path: PathBuf,
    log_path: PathBuf,
    account: Option<Account>,
    source: Option<Source>,
    prism_expires: Option<u64>,
    prism_modified: Option<SystemTime>,
    last_prism_check: Instant,
    last_renewal_try: Instant,
    code: Option<DeviceCode>,
    status: String,
    busy: bool,
    playing: bool,
    cancel: Arc<AtomicBool>,
    tab: ui::Tab,
    styled: bool,
    logo: Option<egui::TextureHandle>,
    new_server: settings::ServerEntry,
    log_text: String,
    log_read: Option<Instant>,
    tx: Notifier,
    epoch: Arc<std::sync::atomic::AtomicU64>,
    rx: Receiver<Event>,
    update: Option<update::Release>,
    shots: Option<ui::Shots>,
    heads_dir: PathBuf,
    head_pixels: std::collections::HashMap<String, Vec<u8>>,
    head_textures: std::collections::HashMap<String, egui::TextureHandle>,
    head_asked: std::collections::HashSet<String>,
    pending_play: Option<(bool, game::Target)>,
    crash: Option<crash::Report>,
    probes: std::collections::HashMap<String, (status::Probe, Instant)>,
    worlds: Vec<worlds::World>,
    worlds_at: Option<Instant>,
    checks: Vec<check::Check>,
    window_cmd: Option<egui::ViewportCommand>,
    play_list: usize,
    add_server: bool,
    diagnosing: bool,
    fields_typed: bool,
}

impl App {
    fn new() -> Self {
        let paths = BelumiPaths::new();
        let settings_path = paths.config_dir().join("launcher.json");
        let settings = Settings::load(&settings_path);
        let (raw_tx, rx) = channel();
        let tx = Notifier { tx: raw_tx, ctx: Arc::new(std::sync::Mutex::new(None)) };
        let mut settings = settings;
        for saved in &mut settings.accounts {
            if saved.refresh_token.is_empty() {
                if let Some(token) = secret::lookup(&saved.uuid) {
                    saved.refresh_token = token;
                }
            }
        }
        if settings.active_saved().is_none() {
            settings.active_account = settings
                .accounts
                .first()
                .map_or_else(String::new, |a| a.uuid.clone());
        }
        let mut app = Self {
            settings,
            settings_path,
            log_path: paths.logs_dir().join("game.log"),
            account: None,
            source: None,
            prism_expires: None,
            prism_modified: None,
            last_prism_check: Instant::now(),
            last_renewal_try: Instant::now(),
            code: None,
            status: "Not signed in.".into(),
            busy: false,
            playing: false,
            cancel: Arc::new(AtomicBool::new(false)),
            tab: ui::Tab::Play,
            styled: false,
            logo: None,
            new_server: settings::ServerEntry::default(),
            log_text: String::new(),
            log_read: None,
            tx,
            epoch: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            rx,
            update: None,
            shots: ui::Shots::from_env(),
            heads_dir: paths.config_dir().join("heads"),
            head_pixels: Default::default(),
            head_textures: Default::default(),
            head_asked: Default::default(),
            pending_play: None,
            crash: None,
            probes: Default::default(),
            worlds: Vec::new(),
            worlds_at: None,
            checks: Vec::new(),
            window_cmd: None,
            play_list: 0,
            add_server: false,
            diagnosing: false,
            fields_typed: false,
        };
        app.check_for_update();
        if app.settings.active_saved().is_some() && !app.settings.client_id.trim().is_empty() {
            app.renew();
        } else if app.settings.use_prism {
            app.import_prism(false);
        }
        app
    }

    fn check_for_update(&mut self) {
        if !self.settings.check_updates {
            return;
        }
        let channel = update::Channel::parse(&self.settings.update_channel);
        let tx = self.tx.clone();
        std::thread::spawn(move || {
            let found = update::check(channel, env!("CARGO_PKG_VERSION")).unwrap_or(None);
            let _ = tx.send(Event::Update(found));
        });
    }

    fn head_texture(&mut self, ctx: &egui::Context, uuid: &str) -> Option<egui::TextureHandle> {
        if uuid.is_empty() {
            return None;
        }
        if !self.head_asked.contains(uuid) {
            self.head_asked.insert(uuid.to_owned());
            if let Some(cached) = heads::cached(&self.heads_dir, uuid) {
                self.head_pixels.insert(uuid.to_owned(), cached);
            }
            if heads::is_stale(&self.heads_dir, uuid) {
                let (tx, dir, uuid, ctx) =
                    (self.tx.clone(), self.heads_dir.clone(), uuid.to_owned(), ctx.clone());
                std::thread::spawn(move || {
                    let head = heads::fetch_and_store(&dir, &uuid);
                    let _ = tx.send(Event::Head(uuid, head));
                    ctx.request_repaint();
                });
            }
        }
        if let Some(texture) = self.head_textures.get(uuid) {
            return Some(texture.clone());
        }
        let pixels = self.head_pixels.get(uuid)?;
        let image = egui::ColorImage::from_rgba_unmultiplied([heads::SIZE, heads::SIZE], pixels);
        let texture = ctx.load_texture(format!("head-{uuid}"), image, egui::TextureOptions::NEAREST);
        self.head_textures.insert(uuid.to_owned(), texture.clone());
        Some(texture)
    }

    fn save(&mut self) {
        let mut on_disk = self.settings.clone();
        for saved in &mut on_disk.accounts {
            if !saved.refresh_token.is_empty() && secret::store(&saved.uuid, &saved.refresh_token) {
                saved.refresh_token.clear();
            }
        }
        if let Err(e) = on_disk.save(&self.settings_path) {
            self.status = format!("Could not save the settings: {e}");
        }
    }

    fn client_id(&self) -> Result<String, String> {
        let id = self.settings.client_id.trim();
        if id.is_empty() {
            Err("Enter your Azure application (client) ID first.".into())
        } else {
            Ok(id.to_owned())
        }
    }

    fn sign_in(&mut self) {
        let client_id = match self.client_id() {
            Ok(id) => id,
            Err(e) => return self.status = e,
        };
        self.busy = true;
        self.cancel.store(true, Ordering::Relaxed);
        self.cancel = Arc::new(AtomicBool::new(false));
        let mine = self.epoch.fetch_add(1, Ordering::SeqCst) + 1;
        let epoch = self.epoch.clone();
        self.status = i18n::tr("Asking Microsoft for a sign-in code...");
        let (tx, cancel) = (self.tx.clone(), self.cancel.clone());
        std::thread::spawn(move || {
            let result = (|| {
                let code = auth::start_device_code(&client_id)?;
                os::open_url(&code.verification_uri);
                let _ = tx.send(Event::Code(code.clone()));
                let ms =
                    auth::wait_for_sign_in(&client_id, &code, &|| cancel.load(Ordering::Relaxed))?;
                let _ = tx.send(Event::Status(
                    "Signing in to Xbox Live and Minecraft...".into(),
                ));
                if cancel.load(Ordering::Relaxed) {
                    return Err("Cancelled".to_owned());
                }
                auth::minecraft_account(&ms)
            })();
            if epoch.load(Ordering::SeqCst) != mine {
                return;
            }
            let _ = tx.send(match result {
                Ok(account) => Event::SignedIn(account),
                Err(e) => Event::Failed(e),
            });
        });
    }

    fn renew(&mut self) {
        let (Ok(client_id), Some(saved)) = (self.client_id(), self.settings.active_saved().cloned())
        else {
            return;
        };
        self.busy = true;
        self.status = i18n::trf("Renewing the sign-in of {}...", &[&saved.name]);
        let tx = self.tx.clone();
        let mine = self.epoch.fetch_add(1, Ordering::SeqCst) + 1;
        let epoch = self.epoch.clone();
        std::thread::spawn(move || {
            let token = if saved.refresh_token.is_empty() {
                secret::lookup(&saved.uuid).unwrap_or_default()
            } else {
                saved.refresh_token.clone()
            };
            let result = if token.is_empty() {
                Err(i18n::tr("The saved sign-in could not be read from the keyring. Sign in again."))
            } else {
                auth::refresh(&client_id, &token).and_then(|ms| auth::minecraft_account(&ms))
            };
            if epoch.load(Ordering::SeqCst) != mine {
                return;
            }
            let _ = tx.send(match result {
                Ok(account) => Event::SignedIn(account),
                Err(e) => Event::Failed(e),
            });
        });
    }

    fn import_prism(&mut self, quiet: bool) {
        self.last_prism_check = Instant::now();
        match prism::load(&self.settings.prism_accounts_path, unix_now()) {
            Ok(p) => {
                let changed = self.account.as_ref().map(|a| a.access_token.as_str())
                    != Some(p.access_token.as_str());
                self.prism_expires = Some(p.expires_at);
                self.prism_modified = p.modified;
                self.source = Some(Source::Prism);
                if changed || !quiet {
                    self.status = format!("Signed in as {} (via Prism).", p.name);
                }
                self.account = Some(Account {
                    name: p.name,
                    uuid: p.uuid,
                    access_token: p.access_token,
                    refresh_token: String::new(),
                    expires_at: p.expires_at,
                });
            }
            Err(e) => {
                if self.source == Some(Source::Prism) {
                    self.account = None;
                    self.source = None;
                    self.prism_expires = None;
                }
                if !quiet || self.account.is_none() {
                    self.status = e;
                }
            }
        }
    }

    fn watch_prism(&mut self) {
        if !self.settings.use_prism
            || self.source == Some(Source::Own)
            || self.busy
            || self.last_prism_check.elapsed() < Duration::from_secs(3)
        {
            return;
        }
        self.last_prism_check = Instant::now();
        let file_changed = prism::candidate_paths(&self.settings.prism_accounts_path)
            .iter()
            .find_map(|p| std::fs::metadata(p).and_then(|m| m.modified()).ok())
            != self.prism_modified;
        let ending = self.prism_expires.is_none_or(|exp| exp < unix_now() + 300);
        if file_changed || ending || self.account.is_none() {
            self.import_prism(true);
        }
    }

    fn sign_out(&mut self) {
        let was_prism = self.source == Some(Source::Prism);
        self.settings.use_prism = false;
        self.source = None;
        self.prism_expires = None;
        self.account = None;
        self.epoch.fetch_add(1, Ordering::SeqCst);
        self.busy = false;
        if !was_prism && let Some(uuid) = self.settings.active_saved().map(|a| a.uuid.clone()) {
            self.settings.forget(&uuid);
            secret::clear(&uuid);
        }
        self.code = None;
        self.cancel.store(true, Ordering::Relaxed);
        self.status = i18n::tr("Signed out.");
        self.save();
        if self.settings.active_saved().is_some() {
            self.renew();
        }
    }

    fn switch_account(&mut self, uuid: &str) -> bool {
        if self.busy || self.settings.accounts.iter().all(|a| a.uuid != uuid) {
            return false;
        }
        self.settings.use_prism = false;
        self.settings.active_account = uuid.to_owned();
        self.account = None;
        self.source = None;
        self.prism_expires = None;
        self.save();
        self.renew();
        true
    }

    fn remove_account(&mut self, uuid: &str) {
        self.epoch.fetch_add(1, Ordering::SeqCst);
        self.busy = false;
        if self.settings.active_account == uuid {
            self.account = None;
            self.source = None;
        }
        if self.settings.forget(uuid).is_some() {
            secret::clear(uuid);
            self.status = "Account removed.".into();
        }
        self.save();
        if self.account.is_none() && self.settings.active_saved().is_some() {
            self.renew();
        }
    }

    fn watch_renewal(&mut self) {
        let due = self.source == Some(Source::Own)
            && !self.busy
            && self.last_renewal_try.elapsed() > Duration::from_secs(120)
            && self
                .account
                .as_ref()
                .is_some_and(|a| a.expires_at <= unix_now() + 900);
        if due {
            self.last_renewal_try = Instant::now();
            self.renew();
        }
    }

    fn play(&mut self, online: bool, target: game::Target) {
        if online && self.source == Some(Source::Prism) {
            self.import_prism(false);
        }
        if online
            && matches!(target, game::Target::Server)
            && let Some(server) = self.settings.servers.get(self.settings.selected_server)
            && !server.account.is_empty()
            && self.settings.active_account != server.account
            && self.settings.accounts.iter().any(|a| a.uuid == server.account)
        {
            let wanted = server.account.clone();
            if self.switch_account(&wanted) {
                self.pending_play = Some((online, target));
                self.status = i18n::tr("Switching to the account of this server...");
                return;
            }
        }
        let account = if online {
            match &self.account {
                Some(account) => Some(account.clone()),
                None => return self.status = i18n::tr("Sign in first (or play offline)."),
            }
        } else {
            None
        };
        if let Some(dir) = self.log_path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        self.crash = None;
        let log = std::fs::File::create(&self.log_path).ok();
        match game::launch(&self.settings, account.as_ref(), &target, log) {
            Ok(mut child) => {
                self.playing = true;
                self.status = i18n::tr("Game started.");
                let tx = self.tx.clone();
                let _ = tx.send(Event::Started);
                match self.settings.after_start.as_str() {
                    "minimize" => self.window_cmd = Some(egui::ViewportCommand::Minimized(true)),
                    "close" => self.window_cmd = Some(egui::ViewportCommand::Close),
                    _ => {}
                }
                std::thread::spawn(move || {
                    let (text, code) = match child.wait() {
                        Ok(status) => (format!("The game ended ({status})."), status.code()),
                        Err(e) => (format!("Lost the game process: {e}"), None),
                    };
                    let _ = tx.send(Event::Exited(text, code));
                });
            }
            Err(e) => self.status = e,
        }
    }

    fn start_diagnose(&mut self) {
        if self.diagnosing || self.playing {
            return;
        }
        self.diagnosing = true;
        self.status = i18n::tr("Diagnose is running; the game opens a few times.");
        let (settings, account, tx) = (self.settings.clone(), self.account.clone(), self.tx.clone());
        let out = cli::report_folder(None, &BelumiPaths::new().data_dir().join("belumi-report"));
        std::thread::spawn(move || {
            let _ = std::fs::create_dir_all(&out);
            let _ = std::fs::write(out.join(".belumi-report"), b"");
            let server = cli::diagnose_server();
            let progress = |line: &str| {
                let _ = tx.send(Event::Status(line.to_owned()));
            };
            let result = diagnose::run(&settings, account.as_ref(), server.as_deref(), &out, &progress)
                .map(|_| out.clone())
                .map_err(|e| e.to_string());
            let _ = tx.send(Event::Diagnosed(result));
        });
    }

    fn probe_servers(&mut self) {
        let due: Vec<String> = self
            .settings
            .servers
            .iter()
            .map(|s| s.address.trim().to_owned())
            .filter(|a| !a.is_empty())
            .filter(|a| {
                self.probes
                    .get(a)
                    .is_none_or(|(p, at)| !matches!(p, status::Probe::Checking) && at.elapsed() >= Duration::from_secs(45))
            })
            .collect();
        for address in due {
            self.probes.insert(address.clone(), (status::Probe::Checking, Instant::now()));
            let tx = self.tx.clone();
            std::thread::spawn(move || {
                let found = status::ping(&address, Duration::from_secs(3));
                let _ = tx.send(Event::Probed(address, found));
            });
        }
    }

    fn refresh_worlds(&mut self) {
        if self.worlds_at.is_some_and(|t| t.elapsed() < Duration::from_secs(5)) {
            return;
        }
        self.worlds_at = Some(Instant::now());
        self.worlds = worlds::scan(&worlds::game_dir(&self.settings.game_dir));
    }

    fn poll(&mut self) {
        while let Ok(event) = self.rx.try_recv() {
            match event {
                Event::Status(s) => self.status = s,
                Event::Code(code) => {
                    self.status = "Enter the code on the Microsoft page that just opened.".into();
                    self.code = Some(code);
                }
                Event::SignedIn(account) => {
                    self.busy = false;
                    self.code = None;
                    self.status = i18n::trf("Signed in as {}.", &[&account.name]);
                    self.settings.remember(SavedAccount {
                        name: account.name.clone(),
                        uuid: account.uuid.clone(),
                        refresh_token: account.refresh_token.clone(),
                    });
                    self.account = Some(account);
                    self.source = Some(Source::Own);
                    self.save();
                    if let Some((online, target)) = self.pending_play.take() {
                        self.pending_play = None;
                        let wanted = self.settings.servers.get(self.settings.selected_server).map(|s| s.account.clone());
                        if wanted.as_deref().is_none_or(|w| w.is_empty() || w == self.settings.active_account) {
                            self.play(online, target);
                        }
                    }
                }
                Event::Failed(e) => {
                    self.pending_play = None;
                    self.busy = false;
                    self.code = None;
                    self.status = e.clone();
                    if self.settings.use_prism && self.account.is_none() {
                        self.import_prism(false);
                        if self.account.is_some() {
                            self.status = format!("{} ({e})", self.status);
                        }
                    }
                }
                Event::Head(uuid, head) => {
                    if let Some(head) = head {
                        self.head_textures.remove(&uuid);
                        self.head_pixels.insert(uuid, head);
                    }
                }
                Event::Started => {}
                Event::Update(release) => self.update = release,
                Event::Exited(text, code) => {
                    self.playing = false;
                    self.status = text;
                    let log = ui::tail(&self.log_path, 256 * 1024);
                    self.crash = crash::from_log(code, &log);
                    if self.crash.is_some() {
                        self.status = i18n::tr("The game stopped unexpectedly.");
                    }
                }
                Event::Diagnosed(result) => {
                    self.diagnosing = false;
                    self.status = match result {
                        Ok(dir) => {
                            os::open(&dir);
                            i18n::trf("Report ready: {}", &[&dir.display().to_string()])
                        }
                        Err(e) => e,
                    };
                }
                Event::Probed(address, found) => {
                    let probe = found.map_or(status::Probe::Down, status::Probe::Up);
                    self.probes.insert(address, (probe, Instant::now()));
                }
            }
        }
    }
}

fn main() -> eframe::Result {
    if let Some(code) = cli::run(std::env::args().skip(1)) {
        std::process::exit(code);
    }
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_app_id("belumi-launcher")
            .with_icon(std::sync::Arc::new(
                eframe::icon_data::from_png_bytes(include_bytes!("../assets/logo-256.png"))
                    .unwrap_or_default(),
            ))
            .with_inner_size([820.0, 600.0])
            .with_min_inner_size([700.0, 520.0]),
        ..Default::default()
    };
    eframe::run_native("Belumi", options, Box::new(|_| Ok(Box::new(App::new()))))
}
