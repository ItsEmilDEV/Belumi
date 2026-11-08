#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

/// The version of this launcher: the release's (`0.1.0-beta.6`, set by the release build as `BELUMI_VERSION`), else Cargo's.
pub const VERSION: &str = match option_env!("BELUMI_VERSION") {
    Some(v) if !v.is_empty() => v,
    _ => env!("CARGO_PKG_VERSION"),
};

mod auth;
mod cli;
mod check;
mod crash;
mod diagnose;
mod fonts;
mod game;
mod heads;
mod icons;
mod join;
mod plugins;
mod i18n;
mod kit;
mod os;
mod prism;
mod secret;
mod switch;
mod settings;
mod skin3d;
mod worlds;
mod status;
mod theme;
mod ui;
mod shaders;
mod mojang;
mod setup;
mod uninstall;
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
    Updated(Result<(), String>),
    Shaders(Result<String, String>),
    PluginList(Result<Vec<plugins::Entry>, String>),
    PluginDone(Result<String, String>),
    Head(String, Option<Vec<u8>>),
    SetupLine(String),
    SetupDone(Result<PathBuf, String>),
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Source {
    Own,
    Prism,
}

/// Runs `work` on a thread; a panic reports a failure instead of leaving the window busy forever.
fn guarded(tx: Notifier, work: impl FnOnce() + Send + 'static) {
    std::thread::spawn(move || {
        if std::panic::catch_unwind(std::panic::AssertUnwindSafe(work)).is_err() {
            let _ = tx.send(Event::Failed(i18n::tr("Something went wrong inside the launcher. Try again.")));
        }
    });
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

struct App {
    settings: Settings,
    /// Discord Rich Presence of the launcher; `None` when Discord is switched off for this run.
    presence: Option<belumi_presence::Presence>,
    opened_at: u64,
    settings_path: PathBuf,
    log_path: PathBuf,
    account: Option<Account>,
    source: Option<Source>,
    prism_expires: Option<u64>,
    prism_modified: Option<SystemTime>,
    last_prism_check: Instant,
    last_renewal_try: Instant,
    renewal_gap: Duration,
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
    /// The update is being downloaded and put in place.
    updating: bool,
    update_progress: Arc<update::Progress>,
    /// The update is in place; the launcher has to start again to use it.
    update_done: Option<String>,
    update_error: Option<String>,
    /// The start-up check found an update the player has not been told about yet.
    update_announced: bool,
    /// The Solas download runs.
    shaders_busy: bool,
    /// Downloading and preparing the game files.
    setup_busy: bool,
    setup_line: String,
    setup_cancel: Arc<AtomicBool>,
    setup_error: Option<String>,
    plugin_list: Vec<plugins::Entry>,
    plugin_busy: bool,
    /// The Solas pack's file name when the game folder has one; looked at again every few seconds.
    solas: Option<String>,
    solas_checked: Option<Instant>,
    /// The player pressed "Check now" and waits for an answer.
    update_manual: bool,
    shots: Option<ui::Shots>,
    heads_dir: PathBuf,
    head_pixels: std::collections::HashMap<String, Vec<u8>>,
    head_textures: std::collections::HashMap<String, egui::TextureHandle>,
    head_asked: std::collections::HashSet<String>,
    skins: std::collections::HashMap<String, skin3d::Skin>,
    pending_play: Option<(bool, game::Target)>,
    /// A `belumi://join/...` link waiting for the player's answer.
    join_request: Option<String>,
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
    uninstall_scope: uninstall::Scope,
    uninstall_worlds: bool,
    /// The plan the user is asked to confirm.
    uninstall_plan: Option<uninstall::Plan>,
}

impl Drop for App {
    fn drop(&mut self) {
        if let Some(presence) = &self.presence {
            presence.stop();
        }
    }
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
            renewal_gap: Duration::from_secs(120),
            code: None,
            status: "Not signed in.".into(),
            busy: false,
            playing: false,
            presence: if belumi_presence::disabled_by_env() {
                None
            } else {
                belumi_presence::app_id().and_then(|id| belumi_presence::Presence::start(&id))
            },
            opened_at: unix_now(),
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
            updating: false,
            update_progress: Arc::new(update::Progress::default()),
            update_done: None,
            update_error: None,
            update_announced: false,
            shaders_busy: false,
            setup_busy: false,
            setup_line: String::new(),
            setup_cancel: Arc::new(AtomicBool::new(false)),
            setup_error: None,
            plugin_list: Vec::new(),
            plugin_busy: false,
            solas: None,
            solas_checked: None,
            update_manual: false,
            shots: ui::Shots::from_env(),
            heads_dir: paths.config_dir().join("heads"),
            head_pixels: Default::default(),
            head_textures: Default::default(),
            head_asked: Default::default(),
            skins: Default::default(),
            pending_play: None,
            join_request: None,
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
            uninstall_scope: uninstall::Scope::Cache,
            uninstall_worlds: false,
            uninstall_plan: None,
        };
        update::clean_up_old();
        app.check_for_update(false);
        if app.settings.active_saved().is_some() && !app.settings.client_id.trim().is_empty() {
            app.renew();
        } else if app.settings.use_prism {
            app.import_prism(false);
        }
        app
    }

    /// Asks GitHub for a newer release; `forced` also when the player switched the start-up check off.
    fn check_for_update(&mut self, forced: bool) {
        if !self.settings.check_updates && !forced {
            return;
        }
        let channel = update::Channel::parse(&self.settings.update_channel);
        let tx = self.tx.clone();
        std::thread::spawn(move || {
            let found = update::check(channel, VERSION).unwrap_or(None);
            let _ = tx.send(Event::Update(found));
        });
    }

    fn shader_dir(&self) -> std::path::PathBuf {
        shaders::dir_of(&worlds::game_dir(&self.settings.game_dir))
    }

    /// Whether the game folder has Solas, looked up at most every five seconds.
    fn refresh_solas(&mut self) {
        if self.solas_checked.is_some_and(|t| t.elapsed() < Duration::from_secs(5)) {
            return;
        }
        self.solas_checked = Some(Instant::now());
        self.solas = shaders::installed(&self.shader_dir());
    }

    fn plugin_dir(&self) -> std::path::PathBuf {
        worlds::game_dir(&self.settings.game_dir)
    }

    /// Fetches the list of plugins from the address in the settings.
    fn refresh_plugin_list(&mut self) {
        let url = self.settings.plugin_index_url.trim().to_owned();
        if self.plugin_busy || url.is_empty() {
            return;
        }
        self.plugin_busy = true;
        self.status = i18n::tr("Fetching the plugin list...");
        let tx = self.tx.clone();
        guarded(self.tx.clone(), move || {
            let result = plugins::http_fetch(&url, 1 << 20)
                .and_then(|bytes| String::from_utf8(bytes).map_err(|_| "The plugin list is not text.".to_owned()))
                .and_then(|text| plugins::parse_index(&text));
            let _ = tx.send(Event::PluginList(result));
        });
    }

    /// Installs (or updates) one plugin of the list.
    fn install_plugin(&mut self, entry: plugins::Entry) {
        if self.plugin_busy {
            return;
        }
        self.plugin_busy = true;
        self.status = i18n::trf("Installing {}...", &[&entry.name]);
        let (tx, dir) = (self.tx.clone(), self.plugin_dir());
        guarded(self.tx.clone(), move || {
            let result = plugins::install(&entry, &dir, &plugins::http_fetch).map(|()| i18n::trf("Installed {}. The game loads it by itself.", &[&entry.name]));
            let _ = tx.send(Event::PluginDone(result));
        });
    }

    fn remove_plugin(&mut self, name: &str) {
        self.status = match plugins::remove(&self.plugin_dir(), name) {
            Ok(()) => i18n::trf("Removed {}.", &[name]),
            Err(e) => e,
        };
    }

    fn switch_plugin(&mut self, name: &str, on: bool) {
        if let Err(e) = plugins::set_enabled(&self.plugin_dir(), name, on) {
            self.status = e;
        }
    }

    /// Downloads the Solas shader pack from Modrinth (after the player asked for it).
    fn download_solas(&mut self) {
        if self.shaders_busy {
            return;
        }
        self.shaders_busy = true;
        self.settings.solas_asked = true;
        let _ = self.settings.save(&self.settings_path);
        self.status = i18n::tr("Downloading the Solas shader pack from Modrinth...");
        let (tx, dir) = (self.tx.clone(), self.shader_dir());
        std::thread::spawn(move || {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| shaders::download(&dir)))
                .unwrap_or_else(|_| Err("Something went wrong while downloading.".to_owned()));
            let _ = tx.send(Event::Shaders(result));
        });
    }

    /// Downloads the game files from Mojang and prepares them (the banner's button).
    fn start_setup(&mut self) {
        if self.setup_busy {
            return;
        }
        self.setup_busy = true;
        self.setup_error = None;
        self.setup_line = i18n::tr("Starting...");
        self.setup_cancel.store(false, Ordering::Relaxed);
        let (tx, cancel, data) = (self.tx.clone(), self.setup_cancel.clone(), BelumiPaths::new().data_dir().clone());
        std::thread::spawn(move || {
            let log_tx = tx.clone();
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                setup::run(&data, &cancel, &|line| {
                    let _ = log_tx.send(Event::SetupLine(line));
                })
            }))
            .unwrap_or_else(|_| Err("Something went wrong while preparing the game.".to_owned()));
            let _ = tx.send(Event::SetupDone(result));
        });
    }

    /// The update button: downloads the release, checks it and puts it next to the running launcher.
    fn start_update(&mut self) {
        let Some(release) = self.update.clone() else {
            return;
        };
        if self.updating {
            return;
        }
        self.updating = true;
        self.update_error = None;
        self.update_progress = Arc::new(update::Progress::default());
        self.status = i18n::trf("Downloading {}...", &[&release.tag]);
        let (tx, progress) = (self.tx.clone(), self.update_progress.clone());
        std::thread::spawn(move || {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| update::install(&release, &progress)))
                .unwrap_or_else(|_| Err("Something went wrong while updating.".to_owned()));
            let _ = tx.send(Event::Updated(result));
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

    fn ensure_skin(&mut self, ctx: &egui::Context, uuid: &str) {
        if self.skins.contains_key(uuid) {
            return;
        }
        let rgba = heads::skin_cached(&self.heads_dir, uuid).unwrap_or_else(|| skin3d::placeholder(kit::t().accent));
        self.skins.insert(uuid.to_owned(), skin3d::Skin::new(ctx, &format!("skin-{uuid}"), &rgba));
    }

    fn uninstall_context(&self) -> uninstall::Context {
        let paths = BelumiPaths::new();
        let game = self.settings.game_dir.trim();
        uninstall::Context {
            exe_dir: std::env::current_exe().ok().and_then(|e| e.parent().map(std::path::Path::to_path_buf)),
            config_dir: paths.config_dir().clone(),
            data_dir: paths.data_dir().clone(),
            game_dir: (!game.is_empty()).then(|| std::path::PathBuf::from(game)),
            home: std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" }).map(std::path::PathBuf::from).unwrap_or_default(),
            worlds: self.uninstall_worlds,
        }
    }

    /// Deletes what the confirmed plan lists. After the app or all data went, the launcher ends at once (it must not write
    /// its settings back).
    fn run_uninstall(&mut self, plan: uninstall::Plan) {
        let ctx = self.uninstall_context();
        let report = uninstall::run(&plan, &ctx.home);
        let gone_for_good = self.uninstall_scope != uninstall::Scope::Cache;
        if gone_for_good {
            if let Some(unins) = &plan.uninstaller {
                let _ = std::process::Command::new(unins).spawn();
            }
            if report.failed.is_empty() {
                std::process::exit(0);
            }
        }
        self.status = if report.failed.is_empty() {
            i18n::tr("Removed {} items.").replace("{}", &report.removed.to_string())
        } else {
            let (path, why) = &report.failed[0];
            i18n::tr("Removed {} items, {} failed (first: {}: {}). Installed by a package manager? Remove it with that.")
                .replacen("{}", &report.removed.to_string(), 1)
                .replacen("{}", &report.failed.len().to_string(), 1)
                .replacen("{}", &path.display().to_string(), 1)
                .replacen("{}", why, 1)
        };
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
        guarded(self.tx.clone(), move || {
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
            })()
            .inspect(cli::remember_session);
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
        guarded(self.tx.clone(), move || {
            let token = if saved.refresh_token.is_empty() {
                secret::lookup(&saved.uuid).unwrap_or_default()
            } else {
                saved.refresh_token.clone()
            };
            let result = if token.is_empty() {
                Err(i18n::tr("The saved sign-in could not be read from the keyring. Sign in again."))
            } else if let Some(mut account) = cli::cached_session(&saved) {
                account.refresh_token = token;
                Ok(account)
            } else {
                auth::refresh(&client_id, &token)
                    .and_then(|ms| auth::minecraft_account(&ms))
                    .inspect(cli::remember_session)
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
            && self.last_renewal_try.elapsed() > self.renewal_gap
            && self
                .account
                .as_ref()
                .is_some_and(|a| a.expires_at <= unix_now() + 900);
        if due {
            self.last_renewal_try = Instant::now();
            self.renewal_gap = (self.renewal_gap * 2).min(Duration::from_secs(3600));
            self.renew();
        }
    }

    /// The question a join link raises: join, join offline, or not.
    fn handle_join(&mut self, ctx: &egui::Context) {
        let Some(address) = self.join_request.clone() else { return };
        let mut answer: Option<Option<bool>> = None;
        egui::Window::new(i18n::tr("Join a server"))
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.label(i18n::trf("A link wants to start Belumi on {}.", &[&address]));
                ui.label(i18n::tr("Only join servers you know."));
                ui.horizontal(|ui| {
                    if ui.button(i18n::tr("Join")).clicked() {
                        answer = Some(Some(true));
                    }
                    if ui.button(i18n::tr("Join offline")).clicked() {
                        answer = Some(Some(false));
                    }
                    if ui.button(i18n::tr("Cancel")).clicked() {
                        answer = Some(None);
                    }
                });
            });
        let Some(choice) = answer else { return };
        self.join_request = None;
        let Some(online) = choice else { return };
        let target = game::Target::Address(address);
        if !online || self.account.is_some() {
            self.play(online, target);
        } else if self.busy {
            self.pending_play = Some((true, target));
            self.status = i18n::tr("Signing in, then joining...");
        } else {
            self.status = i18n::tr("Sign in first (or play offline).");
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
        if online {
            switch::write_accounts(&self.settings, &worlds::game_dir(&self.settings.game_dir));
        }
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

    /// What the launcher tells Discord: nothing while the game runs (the game says it itself).
    fn update_presence(&self) {
        let Some(presence) = &self.presence else {
            return;
        };
        if !self.settings.discord_presence || self.playing || self.diagnosing {
            presence.set(None);
            return;
        }
        let state = match (&self.account, self.settings.discord_show_account) {
            (Some(account), true) => format!("Signed in as {}", account.name),
            _ => "Getting ready to play".to_owned(),
        };
        presence.set(Some(serde_json::json!({
            "details": "In the Belumi launcher",
            "state": state,
            "assets": {"large_image": "belumi", "large_text": "Belumi"},
            "timestamps": {"start": self.opened_at},
        })));
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
                    self.renewal_gap = Duration::from_secs(120);
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
                        // Only the saved server's own account matters when that server is what starts.
                        if !matches!(target, game::Target::Server) || wanted.as_deref().is_none_or(|w| w.is_empty() || w == self.settings.active_account) {
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
                        self.skins.remove(&uuid);
                        self.head_pixels.insert(uuid, head);
                    }
                }
                Event::Started => {}
                Event::Update(release) => {
                    if std::mem::take(&mut self.update_manual) && release.is_none() {
                        self.status = i18n::tr("Belumi is up to date.");
                    }
                    if let (Some(found), false) = (&release, self.update_announced) {
                        self.update_announced = true;
                        os::notify(&i18n::tr("Belumi update available"), &i18n::trf("Version {} is ready: open the launcher to install it.", &[&found.tag]));
                        self.status = i18n::trf("Update available: {}", &[&found.tag]);
                    }
                    self.update = release;
                }
                Event::PluginList(result) => {
                    self.plugin_busy = false;
                    match result {
                        Ok(list) => {
                            self.status = i18n::trf("{} plugins in the list.", &[&list.len().to_string()]);
                            self.plugin_list = list;
                        }
                        Err(e) => self.status = e,
                    }
                }
                Event::PluginDone(result) => {
                    self.plugin_busy = false;
                    self.status = match result {
                        Ok(text) => text,
                        Err(e) => e,
                    };
                }
                Event::SetupLine(line) => self.setup_line = line,
                Event::SetupDone(result) => {
                    self.setup_busy = false;
                    match result {
                        Ok(folder) => {
                            self.settings.resources = folder.display().to_string();
                            self.save();
                            self.checks = check::run(&self.settings);
                            self.status = i18n::tr("The game is ready. Press Play.");
                        }
                        Err(e) => {
                            self.status = i18n::tr("Preparing the game failed.");
                            self.setup_error = Some(e);
                        }
                    }
                }
                Event::Shaders(result) => {
                    self.shaders_busy = false;
                    self.solas_checked = None;
                    self.status = match result {
                        Ok(file) => i18n::trf("Installed {}. The game uses it from the next start.", &[&file]),
                        Err(e) => e,
                    };
                }
                Event::Updated(result) => {
                    self.updating = false;
                    match result {
                        Ok(()) => {
                            let tag = self.update.as_ref().map(|r| r.tag.clone()).unwrap_or_default();
                            self.status = i18n::trf("Updated to {}. Restart to use it.", &[&tag]);
                            self.update_done = Some(tag);
                        }
                        Err(e) => {
                            self.status = e.clone();
                            self.update_error = Some(e);
                        }
                    }
                }
                Event::Exited(text, code) => {
                    self.playing = false;
                    self.status = text;
                    let log = ui::tail(&self.log_path, 256 * 1024);
                    self.crash = crash::from_log(code, &log);
                    if self.crash.is_some() {
                        self.status = i18n::tr("The game stopped unexpectedly.");
                    } else if let Some((uuid, address)) = switch::take_request(&worlds::game_dir(&self.settings.game_dir)) {
                        let target = if address.is_empty() { game::Target::MainMenu } else { game::Target::Address(address) };
                        if self.switch_account(&uuid) {
                            self.pending_play = Some((true, target));
                            self.status = i18n::tr("Switching to the account you picked in the game...");
                        }
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
    let join = join::from_args(std::env::args().skip(1));
    eframe::run_native(
        "Belumi",
        options,
        Box::new(move |_| {
            let mut app = App::new();
            app.join_request = join;
            Ok(Box::new(app))
        }),
    )
}
