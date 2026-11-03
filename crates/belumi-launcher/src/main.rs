mod auth;
mod game;
mod prism;
mod secret;
mod settings;
mod ui;

use auth::{Account, DeviceCode};
use belumi_core::paths::BelumiPaths;
use eframe::egui;
use settings::{SavedAccount, Settings};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

enum Event {
    Status(String),
    Code(DeviceCode),
    SignedIn(Account),
    Failed(String),
    Started,
    Exited(String),
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
    tx: Sender<Event>,
    rx: Receiver<Event>,
}

impl App {
    fn new() -> Self {
        let paths = BelumiPaths::new();
        let settings_path = paths.config_dir().join("launcher.json");
        let settings = Settings::load(&settings_path);
        let (tx, rx) = channel();
        let mut settings = settings;
        if let Some(saved) = settings.account.as_mut() {
            if saved.refresh_token.is_empty() {
                match secret::lookup(&saved.uuid) {
                    Some(token) => saved.refresh_token = token,
                    None => settings.account = None,
                }
            }
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
            rx,
        };
        if app.settings.account.is_some() && !app.settings.client_id.trim().is_empty() {
            app.renew();
        } else if app.settings.use_prism {
            app.import_prism(false);
        }
        app
    }

    fn save(&mut self) {
        let mut on_disk = self.settings.clone();
        if let Some(saved) = on_disk.account.as_mut() {
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
        self.cancel.store(false, Ordering::Relaxed);
        self.status = "Asking Microsoft for a sign-in code...".into();
        let (tx, cancel) = (self.tx.clone(), self.cancel.clone());
        std::thread::spawn(move || {
            let result = (|| {
                let code = auth::start_device_code(&client_id)?;
                let _ = std::process::Command::new("xdg-open")
                    .arg(&code.verification_uri)
                    .spawn();
                let _ = tx.send(Event::Code(code.clone()));
                let ms =
                    auth::wait_for_sign_in(&client_id, &code, &|| cancel.load(Ordering::Relaxed))?;
                let _ = tx.send(Event::Status(
                    "Signing in to Xbox Live and Minecraft...".into(),
                ));
                auth::minecraft_account(&ms)
            })();
            let _ = tx.send(match result {
                Ok(account) => Event::SignedIn(account),
                Err(e) => Event::Failed(e),
            });
        });
    }

    fn renew(&mut self) {
        let (Ok(client_id), Some(saved)) = (self.client_id(), self.settings.account.clone()) else {
            return;
        };
        self.busy = true;
        self.status = format!("Renewing the sign-in of {}...", saved.name);
        let tx = self.tx.clone();
        std::thread::spawn(move || {
            let result = auth::refresh(&client_id, &saved.refresh_token)
                .and_then(|ms| auth::minecraft_account(&ms));
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
        self.settings.use_prism = false;
        self.source = None;
        self.prism_expires = None;
        self.account = None;
        if let Some(saved) = self.settings.account.take() {
            secret::clear(&saved.uuid);
        }
        self.code = None;
        self.cancel.store(true, Ordering::Relaxed);
        self.status = "Signed out.".into();
        self.save();
    }

    fn play(&mut self, online: bool) {
        if online && self.source == Some(Source::Prism) {
            self.import_prism(false);
        }
        let account = if online {
            match &self.account {
                Some(account) => Some(account.clone()),
                None => return self.status = "Sign in first (or play offline).".into(),
            }
        } else {
            None
        };
        if let Some(dir) = self.log_path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let log = std::fs::File::create(&self.log_path).ok();
        match game::launch(&self.settings, account.as_ref(), log) {
            Ok(mut child) => {
                self.playing = true;
                self.status = "Game started.".into();
                let tx = self.tx.clone();
                let _ = tx.send(Event::Started);
                std::thread::spawn(move || {
                    let text = match child.wait() {
                        Ok(status) => format!("The game ended ({status})."),
                        Err(e) => format!("Lost the game process: {e}"),
                    };
                    let _ = tx.send(Event::Exited(text));
                });
            }
            Err(e) => self.status = e,
        }
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
                    self.status = format!("Signed in as {}.", account.name);
                    self.settings.account = Some(SavedAccount {
                        name: account.name.clone(),
                        uuid: account.uuid.clone(),
                        refresh_token: account.refresh_token.clone(),
                    });
                    self.account = Some(account);
                    self.source = Some(Source::Own);
                    self.save();
                }
                Event::Failed(e) => {
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
                Event::Started => {}
                Event::Exited(text) => {
                    self.playing = false;
                    self.status = text;
                }
            }
        }
    }
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_app_id("belumi-launcher")
            .with_icon(std::sync::Arc::new(
                eframe::icon_data::from_png_bytes(include_bytes!("../assets/logo-256.png"))
                    .unwrap_or_default(),
            ))
            .with_inner_size([760.0, 500.0])
            .with_min_inner_size([620.0, 420.0]),
        ..Default::default()
    };
    eframe::run_native("Belumi", options, Box::new(|_| Ok(Box::new(App::new()))))
}
