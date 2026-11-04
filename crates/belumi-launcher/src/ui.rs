use crate::{App, Source, unix_now};
use eframe::egui::{self, Color32, CornerRadius, FontId, RichText, Stroke, vec2};
use std::time::{Duration, Instant};

const BG: Color32 = Color32::from_rgb(0x0e, 0x10, 0x15);
const PANEL: Color32 = Color32::from_rgb(0x14, 0x17, 0x1e);
const CARD: Color32 = Color32::from_rgb(0x1b, 0x1f, 0x29);
const CARD_HOVER: Color32 = Color32::from_rgb(0x24, 0x29, 0x36);
const ACCENT: Color32 = Color32::from_rgb(0x7c, 0x6c, 0xff);
const ACCENT_DIM: Color32 = Color32::from_rgb(0x2a, 0x27, 0x52);
const TEXT: Color32 = Color32::from_rgb(0xe6, 0xe8, 0xee);
const MUTED: Color32 = Color32::from_rgb(0x8b, 0x92, 0xa5);
const OK: Color32 = Color32::from_rgb(0x4a, 0xde, 0x80);
const WARN: Color32 = Color32::from_rgb(0xfb, 0xbf, 0x24);

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
    let mut style = (*ctx.global_style()).clone();
    let v = &mut style.visuals;
    v.dark_mode = true;
    v.override_text_color = Some(TEXT);
    v.panel_fill = BG;
    v.window_fill = PANEL;
    v.extreme_bg_color = Color32::from_rgb(0x0a, 0x0c, 0x10);
    v.faint_bg_color = CARD;
    v.selection.bg_fill = ACCENT_DIM;
    v.selection.stroke = Stroke::new(1.0, ACCENT);
    v.hyperlink_color = ACCENT;
    let round = CornerRadius::same(8);
    for w in [
        &mut v.widgets.noninteractive,
        &mut v.widgets.inactive,
        &mut v.widgets.hovered,
        &mut v.widgets.active,
        &mut v.widgets.open,
    ] {
        w.corner_radius = round;
    }
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, Color32::from_rgb(0x24, 0x28, 0x33));
    v.widgets.inactive.bg_fill = CARD;
    v.widgets.inactive.weak_bg_fill = CARD;
    v.widgets.inactive.bg_stroke = Stroke::NONE;
    v.widgets.hovered.bg_fill = CARD_HOVER;
    v.widgets.hovered.weak_bg_fill = CARD_HOVER;
    v.widgets.hovered.bg_stroke = Stroke::new(1.0, ACCENT);
    v.widgets.active.bg_fill = ACCENT_DIM;
    v.widgets.active.weak_bg_fill = ACCENT_DIM;
    style.spacing.item_spacing = vec2(10.0, 10.0);
    style.spacing.button_padding = vec2(14.0, 7.0);
    style.spacing.interact_size.y = 30.0;
    ctx.set_global_style(style);
}

fn card<R>(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    egui::Frame::new()
        .fill(PANEL)
        .corner_radius(CornerRadius::same(12))
        .inner_margin(egui::Margin::same(16))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            add(ui)
        })
        .inner
}

fn heading(ui: &mut egui::Ui, text: &str) {
    ui.label(
        RichText::new(text)
            .font(FontId::proportional(22.0))
            .strong(),
    );
}

fn muted(text: impl Into<String>) -> RichText {
    RichText::new(text.into()).color(MUTED).size(12.5)
}

fn accent_button(text: &str) -> egui::Button<'static> {
    egui::Button::new(
        RichText::new(text.to_owned())
            .strong()
            .color(Color32::WHITE)
            .size(16.0),
    )
    .fill(ACCENT)
    .min_size(vec2(170.0, 42.0))
}

fn avatar(ui: &mut egui::Ui, name: &str, size: f32) {
    let (rect, _) = ui.allocate_exact_size(vec2(size, size), egui::Sense::hover());
    ui.painter()
        .circle_filled(rect.center(), size / 2.0, ACCENT_DIM);
    ui.painter()
        .circle_stroke(rect.center(), size / 2.0, Stroke::new(1.5, ACCENT));
    let initial = name.chars().next().map_or('?', |c| c.to_ascii_uppercase());
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        initial,
        FontId::proportional(size * 0.5),
        TEXT,
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
        if !self.styled {
            apply_theme(ui.ctx());
            self.styled = true;
        }
        self.poll();
        self.watch_prism();
        ui.ctx().request_repaint_after(self.tick());

        egui::Panel::left("nav")
            .exact_size(170.0)
            .frame(
                egui::Frame::new()
                    .fill(PANEL)
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
                        MUTED
                    });
                    let button = egui::Button::new(text)
                        .fill(if selected {
                            ACCENT_DIM
                        } else {
                            Color32::TRANSPARENT
                        })
                        .stroke(Stroke::NONE)
                        .min_size(vec2(ui.available_width(), 36.0));
                    if ui.add(button).clicked() {
                        self.tab = tab;
                        self.log_read = None;
                    }
                }
            });

        egui::Panel::bottom("status")
            .frame(
                egui::Frame::new()
                    .fill(PANEL)
                    .inner_margin(egui::Margin::symmetric(16, 8)),
            )
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let (dot, color) = if self.playing {
                        ("Running", OK)
                    } else if self.busy {
                        ("Working", WARN)
                    } else {
                        ("Idle", MUTED)
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
                    .fill(BG)
                    .inner_margin(egui::Margin::same(20)),
            )
            .show(ui, |ui| match self.tab {
                Tab::Play => self.page_play(ui),
                Tab::Account => self.page_account(ui),
                Tab::Log => self.page_log(ui),
                Tab::Settings => self.page_settings(ui),
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
                    (Some(Source::Own), _) => "via the Belumi app".to_owned(),
                    _ => "signed in".to_owned(),
                };
                (a.name.clone(), via)
            }
            None => match &self.settings.account {
                Some(s) => (s.name.clone(), "saved, not renewed yet".to_owned()),
                None => (
                    "Not signed in".to_owned(),
                    "play offline or sign in under Account".to_owned(),
                ),
            },
        }
    }

    fn page_play(&mut self, ui: &mut egui::Ui) {
        heading(ui, "Play");
        let (name, detail) = self.account_line();
        card(ui, |ui| {
            ui.horizontal(|ui| {
                avatar(ui, &name, 44.0);
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
                            if ui.selectable_label(selected, label).clicked() {
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
                                                    .clicked()
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
                if ui.add_enabled(ready, egui::Button::new("Add")).clicked() {
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
                .clicked()
            {
                self.play(true);
            }
            let offline =
                egui::Button::new(format!("Play offline as {}", self.settings.offline_name))
                    .min_size(vec2(0.0, 42.0));
            if ui.add_enabled(can_play, offline).clicked() {
                self.play(false);
            }
        });
    }

    fn page_account(&mut self, ui: &mut egui::Ui) {
        heading(ui, "Account");
        let (name, detail) = self.account_line();
        card(ui, |ui| {
            ui.horizontal(|ui| {
                avatar(ui, &name, 44.0);
                ui.vertical(|ui| {
                    ui.label(RichText::new(name).strong().size(16.0));
                    ui.label(muted(detail));
                    if let Some(a) = &self.account {
                        ui.label(muted(&a.uuid));
                    }
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
                        .color(ACCENT),
                );
            });
        }
        ui.horizontal_wrapped(|ui| {
            if ui
                .add_enabled(!self.busy, egui::Button::new("Sign in with Microsoft"))
                .clicked()
            {
                self.sign_in();
            }
            if self.busy && ui.button("Cancel").clicked() {
                self.cancel
                    .store(true, std::sync::atomic::Ordering::Relaxed);
                self.busy = false;
                self.code = None;
                self.status = "Cancelled.".into();
            }
            if ui
                .add_enabled(!self.busy, egui::Button::new("Use my Prism account"))
                .clicked()
            {
                self.settings.use_prism = true;
                self.import_prism(false);
                self.save();
            }
            if ui
                .add_enabled(!self.busy, egui::Button::new("Sign out"))
                .clicked()
            {
                self.sign_out();
            }
        });
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
                if ui.button("Refresh").clicked() {
                    self.log_read = None;
                }
            });
        });
        self.read_log();
        card(ui, |ui| {
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
                                    .color(MUTED),
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
                            if ui.selectable_label(channel == c, c.name()).clicked() {
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
