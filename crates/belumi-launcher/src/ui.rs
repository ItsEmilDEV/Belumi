use crate::{App, Source, unix_now};
use eframe::egui::{self, Color32, CornerRadius, FontId, RichText, Stroke, vec2};
use std::time::{Duration, Instant};

use crate::theme::{self, Theme};

fn t() -> Theme {
    theme::theme(theme::current())
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Play,
    Account,
    Log,
    Settings,
}

impl Tab {
    const ALL: [(Tab, &'static str); 4] = [
        (Tab::Play, "Play"),
        (Tab::Account, "Account"),
        (Tab::Log, "Game log"),
        (Tab::Settings, "Settings"),
    ];
}

fn apply_theme(ctx: &egui::Context) {
    let t = t();
    let mut style = (*ctx.global_style()).clone();
    let v = &mut style.visuals;
    v.dark_mode = true;
    v.override_text_color = Some(t.text);
    v.panel_fill = Color32::TRANSPARENT;
    v.window_fill = t.panel;
    v.extreme_bg_color = if t.bevel { Color32::from_rgb(0x1a, 0x1a, 0x1a) } else { Color32::from_black_alpha(90) };
    v.faint_bg_color = t.card;
    v.selection.bg_fill = t.accent_dim;
    v.selection.stroke = Stroke::new(1.0, t.accent);
    v.hyperlink_color = t.accent;
    let round = CornerRadius::same(t.control_radius);
    for w in [
        &mut v.widgets.noninteractive,
        &mut v.widgets.inactive,
        &mut v.widgets.hovered,
        &mut v.widgets.active,
        &mut v.widgets.open,
    ] {
        w.corner_radius = round;
    }
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, t.line);
    v.widgets.inactive.bg_fill = t.control;
    v.widgets.inactive.weak_bg_fill = t.control;
    v.widgets.inactive.bg_stroke = if t.bevel { Stroke::new(t.border, t.line) } else { Stroke::NONE };
    v.widgets.hovered.bg_fill = t.control_hover;
    v.widgets.hovered.weak_bg_fill = t.control_hover;
    v.widgets.hovered.bg_stroke = Stroke::new(t.border.max(1.0), t.accent);
    v.widgets.active.bg_fill = t.accent_dim;
    v.widgets.active.weak_bg_fill = t.accent_dim;
    v.widgets.active.bg_stroke = Stroke::new(t.border.max(1.0), t.accent);
    style.spacing.item_spacing = vec2(10.0, 10.0);
    style.spacing.button_padding = t.button_padding;
    style.spacing.interact_size.y = 30.0;
    ctx.set_global_style(style);
}

/// The window's ground: flat, a gradient for the glass style, a tiled stone for Minecraft.
fn paint_ground(ui: &egui::Ui) {
    let t = t();
    let rect = ui.ctx().content_rect();
    let painter = ui.painter();
    if t.glass {
        let (top, bottom) = (Color32::from_rgb(0x1a, 0x13, 0x3a), Color32::from_rgb(0x08, 0x10, 0x1e));
        let mut mesh = egui::Mesh::default();
        mesh.colored_vertex(rect.left_top(), top);
        mesh.colored_vertex(rect.right_top(), Color32::from_rgb(0x12, 0x18, 0x34));
        mesh.colored_vertex(rect.right_bottom(), bottom);
        mesh.colored_vertex(rect.left_bottom(), Color32::from_rgb(0x0e, 0x14, 0x2a));
        mesh.add_triangle(0, 1, 2);
        mesh.add_triangle(0, 2, 3);
        painter.add(egui::Shape::mesh(mesh));
    } else {
        painter.rect_filled(rect, 0.0, t.bg);
        if t.bevel {
            let tile = 16.0;
            let (cols, rows) = ((rect.width() / tile).ceil() as i32, (rect.height() / tile).ceil() as i32);
            for y in 0..rows {
                for x in 0..cols {
                    let h = (x.wrapping_mul(73856093) ^ y.wrapping_mul(19349663)) & 7;
                    let shade = [0u8, 6, 12, 0, 4, 8, 0, 10][h as usize];
                    if shade > 0 {
                        let at = rect.min + vec2(x as f32 * tile, y as f32 * tile);
                        painter.rect_filled(egui::Rect::from_min_size(at, vec2(tile, tile)), 0.0, Color32::from_white_alpha(shade));
                    }
                }
            }
        }
    }
}

/// A raised or sunken box: light top-left and dark bottom-right edges (Minecraft).
fn bevel_shapes(rect: egui::Rect, fill: Color32, raised: bool) -> Vec<egui::Shape> {
    let t = t();
    let (light, dark) = (Color32::from_white_alpha(70), Color32::from_black_alpha(110));
    let (a, b) = if raised { (light, dark) } else { (dark, light) };
    let w = t.border;
    let edge = |r: egui::Rect, c: Color32| egui::Shape::rect_filled(r, 0.0, c);
    vec![
        egui::Shape::rect_filled(rect, 0.0, fill),
        edge(egui::Rect::from_min_size(rect.min, vec2(rect.width(), w)), a),
        edge(egui::Rect::from_min_size(rect.min, vec2(w, rect.height())), a),
        edge(egui::Rect::from_min_size(egui::pos2(rect.min.x, rect.max.y - w), vec2(rect.width(), w)), b),
        edge(egui::Rect::from_min_size(egui::pos2(rect.max.x - w, rect.min.y), vec2(w, rect.height())), b),
        egui::Shape::rect_stroke(rect, 0.0, Stroke::new(1.0, t.line), egui::StrokeKind::Outside),
    ]
}

/// Buttons in the Minecraft style get raised edges (sunken while pressed); other styles are untouched.
trait Raise {
    fn raised(self, ui: &egui::Ui) -> Self;
}
impl Raise for egui::Response {
    fn raised(self, ui: &egui::Ui) -> Self {
        let t = t();
        if t.bevel && self.rect.width() > 0.0 {
            let w = t.border;
            let pressed = self.is_pointer_button_down_on();
            let (light, dark) = (Color32::from_white_alpha(80), Color32::from_black_alpha(120));
            let (a, b) = if pressed { (dark, light) } else { (light, dark) };
            let r = self.rect;
            let p = ui.painter();
            p.rect_filled(egui::Rect::from_min_size(r.min, vec2(r.width(), w)), 0.0, a);
            p.rect_filled(egui::Rect::from_min_size(r.min, vec2(w, r.height())), 0.0, a);
            p.rect_filled(egui::Rect::from_min_size(egui::pos2(r.min.x, r.max.y - w), vec2(r.width(), w)), 0.0, b);
            p.rect_filled(egui::Rect::from_min_size(egui::pos2(r.max.x - w, r.min.y), vec2(w, r.height())), 0.0, b);
        }
        self
    }
}

fn card<R>(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    let t = t();
    let frame = egui::Frame::new()
        .fill(if t.bevel { Color32::TRANSPARENT } else { t.card })
        .corner_radius(CornerRadius::same(t.card_radius))
        .inner_margin(egui::Margin::same(16));
    let frame = if t.glass { frame.stroke(Stroke::new(1.0, t.line)) } else { frame };
    let slot = ui.painter().add(egui::Shape::Noop);
    let mut prepared = frame.begin(ui);
    prepared.content_ui.set_width(prepared.content_ui.available_width());
    let out = add(&mut prepared.content_ui);
    if t.bevel {
        let rect = prepared.frame.outer_rect(prepared.content_ui.min_rect());
        ui.painter().set(slot, egui::Shape::Vec(bevel_shapes(rect, t.card, true)));
    }
    prepared.end(ui);
    out
}

fn heading(ui: &mut egui::Ui, text: &str) {
    ui.label(
        RichText::new(text)
            .font(FontId::proportional(22.0))
            .strong(),
    );
}

fn subheading(ui: &mut egui::Ui, text: &str) {
    ui.add_space(6.0);
    ui.label(RichText::new(text).font(FontId::proportional(16.0)).strong());
}

fn muted(text: impl Into<String>) -> RichText {
    RichText::new(text.into()).color(t().muted).size(12.5)
}

fn accent_button(text: &str) -> egui::Button<'static> {
    egui::Button::new(
        RichText::new(text.to_owned())
            .strong()
            .color(t().on_accent)
            .size(16.0),
    )
    .fill(t().accent)
    .min_size(vec2(170.0, 42.0))
}

fn avatar(ui: &mut egui::Ui, name: &str, head: Option<&egui::TextureHandle>, size: f32) {
    let (rect, _) = ui.allocate_exact_size(vec2(size, size), egui::Sense::hover());
    if let Some(head) = head {
        egui::Image::new(head).corner_radius(size * 0.12).paint_at(ui, rect);
        return;
    }
    ui.painter()
        .circle_filled(rect.center(), size / 2.0, t().accent_dim);
    ui.painter()
        .circle_stroke(rect.center(), size / 2.0, Stroke::new(1.5, t().accent));
    let initial = name.chars().next().map_or('?', |c| c.to_ascii_uppercase());
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        initial,
        FontId::proportional(size * 0.5),
        t().text,
    );
}

impl App {
    fn tick(&self) -> Duration {
        if self.busy || self.code.is_some() {
            Duration::from_millis(250)
        } else if self.tab == Tab::Log && self.playing {
            Duration::from_millis(500)
        } else {
            Duration::from_secs(if self.settings.use_prism { 2 } else { 10 })
        }
    }

    fn read_log(&mut self) {
        let due = self
            .log_read
            .is_none_or(|t| t.elapsed() >= Duration::from_millis(400));
        if !due {
            return;
        }
        self.log_read = Some(Instant::now());
        self.log_text = tail(&self.log_path, 96 * 1024);
    }
}

fn tail(path: &std::path::Path, bytes: u64) -> String {
    use std::io::{Read, Seek, SeekFrom};
    let Ok(mut file) = std::fs::File::open(path) else {
        return String::new();
    };
    let len = file.metadata().map_or(0, |m| m.len());
    let start = len.saturating_sub(bytes);
    if file.seek(SeekFrom::Start(start)).is_err() {
        return String::new();
    }
    let mut raw = Vec::new();
    let _ = file.read_to_end(&mut raw);
    let text = String::from_utf8_lossy(&raw).into_owned();
    if start > 0 {
        text.split_once('\n')
            .map_or(String::new(), |(_, rest)| rest.to_owned())
    } else {
        text
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let want = theme::Style::parse(&self.settings.style);
        if !self.styled || theme::current() != want {
            theme::set(want);
            apply_theme(ui.ctx());
            self.styled = true;
        }
        paint_ground(ui);
        self.shot_step(ui.ctx());
        self.poll();
        self.watch_prism();
        self.watch_renewal();
        ui.ctx().request_repaint_after(self.tick());

        egui::Panel::left("nav")
            .exact_size(170.0)
            .frame(
                egui::Frame::new()
                    .fill(t().panel)
                    .inner_margin(egui::Margin::same(14)),
            )
            .show(ui, |ui| {
                let logo = self.logo.get_or_insert_with(|| {
                    let icon =
                        eframe::icon_data::from_png_bytes(include_bytes!("../assets/logo-256.png"))
                            .unwrap_or_default();
                    let image = egui::ColorImage::from_rgba_unmultiplied(
                        [icon.width as usize, icon.height as usize],
                        &icon.rgba,
                    );
                    ui.ctx()
                        .load_texture("logo", image, egui::TextureOptions::LINEAR)
                });
                ui.add(egui::Image::new(&*logo).fit_to_exact_size(vec2(64.0, 64.0)));
                ui.label(
                    RichText::new("Belumi")
                        .font(FontId::proportional(20.0))
                        .strong(),
                );
                ui.label(muted("Minecraft 26.2 / 26.3"));
                ui.add_space(14.0);
                for (tab, label) in Tab::ALL {
                    let selected = self.tab == tab;
                    let text = RichText::new(label).size(15.0).color(if selected {
                        Color32::WHITE
                    } else {
                        t().muted
                    });
                    let button = egui::Button::new(text)
                        .fill(if selected {
                            t().accent_dim
                        } else {
                            Color32::TRANSPARENT
                        })
                        .stroke(Stroke::NONE)
                        .min_size(vec2(ui.available_width(), 36.0));
                    if ui.add(button).raised(ui).clicked() {
                        self.tab = tab;
                        self.log_read = None;
                    }
                }
            });

        egui::Panel::bottom("status")
            .frame(
                egui::Frame::new()
                    .fill(t().panel)
                    .inner_margin(egui::Margin::symmetric(16, 8)),
            )
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let (dot, color) = if self.playing {
                        ("Running", t().ok)
                    } else if self.busy {
                        ("Working", t().warn)
                    } else {
                        ("Idle", t().muted)
                    };
                    let (rect, _) = ui.allocate_exact_size(vec2(10.0, 10.0), egui::Sense::hover());
                    ui.painter().circle_filled(rect.center(), 4.0, color);
                    ui.label(RichText::new(dot).color(color).size(12.5));
                    ui.label(muted(&self.status));
                });
            });

        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(Color32::TRANSPARENT)
                    .inner_margin(egui::Margin::same(20)),
            )
            .show(ui, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| match self.tab {
                        Tab::Play => self.page_play(ui),
                        Tab::Account => self.page_account(ui),
                        Tab::Log => self.page_log(ui),
                        Tab::Settings => self.page_settings(ui),
                    });
            });
    }
}

impl App {
    fn account_line(&self) -> (String, String) {
        match &self.account {
            Some(a) => {
                let via = match (self.source, self.prism_expires) {
                    (Some(Source::Prism), Some(exp)) => {
                        let hours = exp.saturating_sub(unix_now()) as f64 / 3600.0;
                        format!("via Prism · token valid for {hours:.1} h")
                    }
                    (Some(Source::Own), _) => {
                        let hours = a.expires_at.saturating_sub(unix_now()) as f64 / 3600.0;
                        format!("via the Belumi app · token valid for {hours:.1} h")
                    }
                    _ => "signed in".to_owned(),
                };
                (a.name.clone(), via)
            }
            None => match self.settings.active_saved() {
                Some(s) => (s.name.clone(), "saved, not renewed yet".to_owned()),
                None => (
                    "Not signed in".to_owned(),
                    "play offline or sign in under Account".to_owned(),
                ),
            },
        }
    }

    fn active_head(&mut self, ctx: &egui::Context) -> Option<egui::TextureHandle> {
        let uuid = match &self.account {
            Some(a) => a.uuid.clone(),
            None => self.settings.active_saved().map(|a| a.uuid.clone())?,
        };
        self.head_texture(ctx, &uuid)
    }

    fn page_play(&mut self, ui: &mut egui::Ui) {
        heading(ui, "Play");
        let (name, detail) = self.account_line();
        let head = self.active_head(ui.ctx());
        card(ui, |ui| {
            ui.horizontal(|ui| {
                avatar(ui, &name, head.as_ref(), 44.0);
                ui.vertical(|ui| {
                    ui.label(RichText::new(name).strong().size(16.0));
                    ui.label(muted(detail));
                });
            });
        });

        if let Some(release) = self.update.clone() {
            card(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(format!("Update available: {}", release.tag)).strong());
                    ui.hyperlink_to("Open the release page", &release.url);
                });
            });
        }
        ui.label(muted("SERVERS"));
        let mut remove = None;
        let mut changed = false;
        card(ui, |ui| {
            egui::ScrollArea::vertical()
                .max_height(150.0)
                .show(ui, |ui| {
                    for (i, server) in self.settings.servers.iter_mut().enumerate() {
                        ui.horizontal(|ui| {
                            let selected = self.settings.selected_server == i;
                            let label =
                                RichText::new(format!("{}   {}", server.name, server.address))
                                    .size(14.5);
                            if ui.selectable_label(selected, label).raised(ui).clicked() {
                                self.settings.selected_server = i;
                                changed = true;
                            }
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if ui.small_button("×").on_hover_text("Remove").clicked() {
                                        remove = Some(i);
                                    }
                                    egui::ComboBox::from_id_salt(("version", i))
                                        .selected_text(crate::versions::label(&server.version))
                                        .width(150.0)
                                        .show_ui(ui, |ui| {
                                            let mut pick = |ui: &mut egui::Ui, id: &str| {
                                                let on = server.version.trim() == id
                                                    || (id == crate::versions::AUTO
                                                        && crate::versions::find(&server.version).is_none());
                                                if ui
                                                    .selectable_label(on, crate::versions::label(id))
                                                    .raised(ui).clicked()
                                                {
                                                    server.version = if id == crate::versions::AUTO {
                                                        String::new()
                                                    } else {
                                                        id.to_owned()
                                                    };
                                                    changed = true;
                                                }
                                            };
                                            pick(ui, crate::versions::AUTO);
                                            for v in crate::versions::VERSIONS {
                                                pick(ui, v.id);
                                            }
                                        });
                                },
                            );
                        });
                    }
                    if self.settings.servers.is_empty() {
                        ui.label(muted("No server yet: the game starts in its main menu."));
                    }
                });
            ui.separator();
            ui.horizontal(|ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut self.new_server.name)
                        .hint_text("Name")
                        .desired_width(120.0),
                );
                ui.add(
                    egui::TextEdit::singleline(&mut self.new_server.address)
                        .hint_text("host:port")
                        .desired_width(200.0),
                );
                let ready = !self.new_server.address.trim().is_empty();
                if ui.add_enabled(ready, egui::Button::new("Add")).raised(ui).clicked() {
                    let mut entry = std::mem::take(&mut self.new_server);
                    entry.address = entry.address.trim().to_owned();
                    if entry.name.trim().is_empty() {
                        entry.name = entry.address.clone();
                    }
                    self.settings.servers.push(entry);
                    self.settings.selected_server = self.settings.servers.len() - 1;
                    changed = true;
                }
            });
        });
        if let Some(i) = remove {
            self.settings.servers.remove(i);
            if self.settings.selected_server >= i && self.settings.selected_server > 0 {
                self.settings.selected_server -= 1;
            }
            changed = true;
        }
        if changed {
            self.save();
        }

        ui.add_space(6.0);
        ui.horizontal(|ui| {
            let can_play = !self.playing && !self.busy;
            let online = can_play && self.account.is_some();
            let text = if self.playing { "Running..." } else { "Play" };
            if ui
                .add_enabled(online, accent_button(text))
                .on_disabled_hover_text("Sign in first, or play offline")
                .raised(ui).clicked()
            {
                self.play(true, true);
            }
            if ui
                .add_enabled(
                    online,
                    egui::Button::new("Main menu").min_size(vec2(0.0, 42.0)),
                )
                .on_hover_text("Starts the game without joining the selected server")
                .on_disabled_hover_text("Sign in first, or play offline")
                .raised(ui).clicked()
            {
                self.play(true, false);
            }
            let offline =
                egui::Button::new(format!("Play offline as {}", self.settings.offline_name))
                    .min_size(vec2(0.0, 42.0));
            if ui.add_enabled(can_play, offline).raised(ui).clicked() {
                self.play(false, true);
            }
        });
    }

    fn page_account(&mut self, ui: &mut egui::Ui) {
        heading(ui, "Account");
        let (name, detail) = self.account_line();
        let head = self.active_head(ui.ctx());
        card(ui, |ui| {
            ui.horizontal(|ui| {
                avatar(ui, &name, head.as_ref(), 44.0);
                ui.vertical(|ui| {
                    ui.label(RichText::new(name).strong().size(16.0));
                    ui.label(muted(detail));
                });
            });
        });
        if let Some(code) = &self.code {
            card(ui, |ui| {
                ui.label(format!("Open {} and enter:", code.verification_uri));
                ui.label(
                    RichText::new(&code.user_code)
                        .font(FontId::monospace(30.0))
                        .strong()
                        .color(t().accent),
                );
            });
        }
        ui.horizontal_wrapped(|ui| {
            if ui
                .add_enabled(!self.busy, egui::Button::new("Add account"))
                .on_hover_text("Signs in another Microsoft account; the others stay saved")
                .raised(ui).clicked()
            {
                self.sign_in();
            }
            if self.busy && ui.button("Cancel").raised(ui).clicked() {
                self.cancel
                    .store(true, std::sync::atomic::Ordering::Relaxed);
                self.busy = false;
                self.code = None;
                self.status = "Cancelled.".into();
            }
            if ui
                .add_enabled(
                    !self.busy && self.settings.active_saved().is_some(),
                    egui::Button::new("Renew now"),
                )
                .on_hover_text("Gets a fresh token for the active account")
                .raised(ui).clicked()
            {
                self.renew();
            }
            if ui
                .add_enabled(!self.busy, egui::Button::new("Use my Prism account"))
                .raised(ui).clicked()
            {
                self.settings.use_prism = true;
                self.import_prism(false);
                self.save();
            }
            if ui
                .add_enabled(!self.busy, egui::Button::new(RichText::new("Sign out").color(t().danger)))
                .raised(ui).clicked()
            {
                self.sign_out();
            }
        });
        if !self.settings.accounts.is_empty() {
            subheading(ui, "Saved accounts");
            let rows: Vec<(String, String, bool)> = self
                .settings
                .accounts
                .iter()
                .map(|a| {
                    (
                        a.name.clone(),
                        a.uuid.clone(),
                        a.uuid == self.settings.active_account
                            && self.source != Some(Source::Prism),
                    )
                })
                .collect();
            let mut switch = None;
            let mut remove = None;
            for (name, uuid, active) in rows {
                let head = self.head_texture(ui.ctx(), &uuid);
                card(ui, |ui| {
                    ui.horizontal(|ui| {
                        avatar(ui, &name, head.as_ref(), 28.0);
                        ui.label(RichText::new(&name).strong());
                        ui.label(muted(if active { "active" } else { "" }));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui
                                .add_enabled(!self.busy, egui::Button::new(RichText::new("Remove").color(t().danger)))
                                .raised(ui).clicked()
                            {
                                remove = Some(uuid.clone());
                            }
                            if !active
                                && ui
                                    .add_enabled(!self.busy, egui::Button::new("Use"))
                                    .raised(ui).clicked()
                            {
                                switch = Some(uuid.clone());
                            }
                        });
                    });
                });
            }
            if let Some(uuid) = switch {
                self.switch_account(&uuid);
            }
            if let Some(uuid) = remove {
                self.remove_account(&uuid);
            }
        }
        if ui
            .checkbox(&mut self.settings.use_prism, "Keep using Prism's login automatically")
            .on_hover_text(
                "Reads only the active Microsoft account's Minecraft token from Prism's accounts.json, \
                 again whenever Prism renews it. Prism has to be opened now and then (about once a day).",
            )
            .changed()
        {
            if self.settings.use_prism {
                self.import_prism(false);
            }
            self.save();
        }
        ui.label(muted("The sign-in is kept in your desktop's keyring when there is one (secret-tool), otherwise in an owner-only file."));
    }

    fn page_log(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            heading(ui, "Game log");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Refresh").raised(ui).clicked() {
                    self.log_read = None;
                }
            });
        });
        self.read_log();
        let solid = egui::Frame::new()
            .fill(if t().bevel { Color32::from_rgb(0x1a, 0x1a, 0x1a) } else { Color32::from_rgb(0x0a, 0x0c, 0x12) })
            .corner_radius(CornerRadius::same(t().card_radius))
            .inner_margin(egui::Margin::same(16))
            .stroke(Stroke::new(1.0, t().line));
        solid.show(ui, |ui| {
            ui.set_width(ui.available_width());
            if self.log_text.is_empty() {
                ui.label(muted(format!(
                    "Nothing yet. The game writes to {}",
                    self.log_path.display()
                )));
            } else {
                egui::ScrollArea::both()
                    .stick_to_bottom(true)
                    .auto_shrink(false)
                    .show(ui, |ui| {
                        ui.add(
                            egui::Label::new(
                                RichText::new(&self.log_text)
                                    .font(FontId::monospace(12.0))
                                    .color(t().muted),
                            )
                            .extend(),
                        );
                    });
            }
        });
    }

    fn page_settings(&mut self, ui: &mut egui::Ui) {
        heading(ui, "Settings");
        let mut changed = false;
        card(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(muted("Look"));
                for style in theme::Style::ALL {
                    let on = self.settings.style == style.id();
                    if ui.selectable_label(on, style.label()).raised(ui).clicked() && !on {
                        self.settings.style = style.id().to_owned();
                        changed = true;
                    }
                }
            });
        });
        card(ui, |ui| {
            egui::Grid::new("settings")
                .num_columns(2)
                .spacing([14.0, 12.0])
                .show(ui, |ui| {
                    let mut field = |ui: &mut egui::Ui, label: &str, value: &mut String| {
                        ui.label(muted(label));
                        changed |= ui
                            .add(egui::TextEdit::singleline(value).desired_width(440.0))
                            .changed();
                        ui.end_row();
                    };
                    field(ui, "Azure client ID", &mut self.settings.client_id);
                    field(ui, "Game binary", &mut self.settings.client_binary);
                    field(ui, "Resources", &mut self.settings.resources);
                    field(ui, "Offline name", &mut self.settings.offline_name);
                    field(
                        ui,
                        "Prism accounts.json",
                        &mut self.settings.prism_accounts_path,
                    );
                });
            ui.separator();
            ui.horizontal(|ui| {
                changed |= ui
                    .checkbox(&mut self.settings.check_updates, "Check for updates at start")
                    .on_hover_text("Asks GitHub's public release list; nothing is installed automatically.")
                    .changed();
                let channel = crate::update::Channel::parse(&self.settings.update_channel);
                egui::ComboBox::from_id_salt("channel")
                    .selected_text(channel.name())
                    .show_ui(ui, |ui| {
                        for c in [crate::update::Channel::Stable, crate::update::Channel::Nightly] {
                            if ui.selectable_label(channel == c, c.name()).raised(ui).clicked() {
                                self.settings.update_channel = c.name().to_owned();
                                changed = true;
                            }
                        }
                    });
            });
        });
        if changed {
            self.save();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tail_cuts_at_a_line_start_and_survives_a_missing_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("game.log");
        assert_eq!(tail(&path, 100), "");
        std::fs::write(&path, "alpha\nbeta\ngamma\n").unwrap();
        assert_eq!(tail(&path, 100), "alpha\nbeta\ngamma\n");
        assert_eq!(tail(&path, 9), "gamma\n");
    }
}

/// A development aid: `BELUMI_LAUNCHER_SHOTS=DIR` walks every style and page, saves a PNG of each
/// into DIR and quits, with two made-up accounts so no real name shows.
pub struct Shots {
    dir: std::path::PathBuf,
    index: usize,
    frames: u32,
    waiting: bool,
    done: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl Shots {
    pub fn from_env() -> Option<Self> {
        let dir = std::path::PathBuf::from(std::env::var_os("BELUMI_LAUNCHER_SHOTS")?);
        std::fs::create_dir_all(&dir).ok()?;
        Some(Self { dir, index: 0, frames: 0, waiting: false, done: Default::default() })
    }
}

const SHOT_PAGES: [Tab; 4] = [Tab::Play, Tab::Account, Tab::Log, Tab::Settings];

impl App {
    pub(crate) fn demo_accounts(&mut self) {
        use crate::settings::SavedAccount;
        let demo = |name: &str, uuid: &str| SavedAccount { name: name.into(), uuid: uuid.into(), refresh_token: "x".into() };
        self.settings.accounts = vec![demo("Steve", "00000000000000000000000000000001"), demo("Alex", "00000000000000000000000000000002")];
        self.settings.active_account = self.settings.accounts[0].uuid.clone();
        self.account = Some(crate::auth::Account {
            name: "Steve".into(),
            uuid: self.settings.active_account.clone(),
            access_token: "x".into(),
            refresh_token: "x".into(),
            expires_at: unix_now() + 3 * 3600,
        });
        self.source = Some(Source::Own);
        self.settings.servers.push(crate::settings::ServerEntry {
            name: "Survival".into(),
            address: "play.example.net".into(),
            version: "26.3".into(),
        });
        self.status = "Signed in as Steve.".into();
        self.log_text = "[18:02:11] Horizon: resources loaded in 2.1s\n[18:02:12] Voice chat: connected\n[18:02:14] Loaded 441 chunks".into();
    }

    fn shot_step(&mut self, ctx: &egui::Context) {
        let Some(mut shots) = self.shots.take() else {
            return;
        };
        if shots.index == 0 && shots.frames == 0 && !shots.waiting {
            self.demo_accounts();
        }
        let total = theme::Style::ALL.len() * SHOT_PAGES.len();
        if shots.waiting && shots.done.swap(false, std::sync::atomic::Ordering::SeqCst) {
            shots.index += 1;
            shots.frames = 0;
            shots.waiting = false;
        }
        if shots.index >= total {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            self.shots = Some(shots);
            return;
        }
        let (style, tab) = (
            theme::Style::ALL[shots.index / SHOT_PAGES.len()],
            SHOT_PAGES[shots.index % SHOT_PAGES.len()],
        );
        self.settings.style = style.id().to_owned();
        self.tab = tab;
        shots.frames += 1;
        if shots.frames == 8 && !shots.waiting {
            shots.waiting = true;
            let name = format!("{}-{}.png", style.id(), ["play", "account", "log", "settings"][tab as usize]);
            let (path, done) = (shots.dir.join(name), shots.done.clone());
            ctx.request_screenshot(move |image| {
                if let Ok(file) = std::fs::File::create(path) {
                    let mut enc = png::Encoder::new(std::io::BufWriter::new(file), image.width() as u32, image.height() as u32);
                    enc.set_color(png::ColorType::Rgba);
                    enc.set_depth(png::BitDepth::Eight);
                    if let Ok(mut w) = enc.write_header() {
                        let _ = w.write_image_data(image.as_raw());
                    }
                }
                done.store(true, std::sync::atomic::Ordering::SeqCst);
            });
        }
        ctx.request_repaint();
        self.shots = Some(shots);
    }
}
