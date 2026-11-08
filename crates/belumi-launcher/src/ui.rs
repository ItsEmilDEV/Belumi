use crate::game::Target;
use crate::i18n::{self, tr, trf};
use crate::icons::Icon;
use crate::kit::{
    Kind, action, apply_theme, avatar, card, dot, empty_state, face, muted, paint_ground, paint_board,
    paint_ledge, section, segmented, t, title, well,
};
use crate::status::Probe;
use crate::theme::{self, Style};
use crate::{App, Source, unix_now, worlds};
use eframe::egui::{self, Color32, CornerRadius, RichText, Sense, Stroke, vec2};
use std::time::{Duration, Instant};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Play,
    Account,
    Log,
    Settings,
}

impl Tab {
    const ALL: [(Tab, Icon, &'static str); 4] = [
        (Tab::Play, Icon::Play, "Play"),
        (Tab::Account, Icon::User, "Account"),
        (Tab::Log, Icon::Log, "Game log"),
        (Tab::Settings, Icon::Settings, "Settings"),
    ];
}

fn nav_item(ui: &mut egui::Ui, icon: Icon, label: &str, selected: bool) -> egui::Response {
    let t = t();
    let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), 40.0), Sense::click());
    let lift = ui.ctx().animate_value_with_time(response.id.with("nav"), if response.hovered() { 1.0 } else { 0.0 }, 0.12);
    let painter = ui.painter();
    if t.bevel {
        let tab = if selected { rect } else { egui::Rect::from_min_max(rect.min, rect.max - vec2(10.0 - 6.0 * lift, 0.0)) };
        let fill = if selected { t.control_hover } else { crate::kit::mix_colors(t.control, t.control_hover, lift) };
        painter.add(egui::Shape::Vec(crate::kit::bevel_shapes(tab.shrink2(vec2(0.0, 1.0)), fill, !selected, 2.0)));
        if selected {
            painter.rect_filled(egui::Rect::from_min_size(tab.min + vec2(6.0, 8.0), vec2(6.0, rect.height() - 16.0)), 0.0, t.accent);
        }
    } else if selected {
    } else if lift > 0.01 {
        painter.rect_filled(rect, f32::from(t.control_radius), Color32::from_white_alpha((14.0 * lift) as u8));
    }
    let color = if selected || t.bevel { t.text } else { t.muted };
    let color = if lift > 0.01 && !selected { Color32::from_rgb(
        (f32::from(color.r()) + (f32::from(t.text.r()) - f32::from(color.r())) * lift) as u8,
        (f32::from(color.g()) + (f32::from(t.text.g()) - f32::from(color.g())) * lift) as u8,
        (f32::from(color.b()) + (f32::from(t.text.b()) - f32::from(color.b())) * lift) as u8,
    ) } else { color };
    crate::icons::paint(painter, icon, egui::Rect::from_center_size(egui::pos2(rect.min.x + if t.bevel { 28.0 } else { 22.0 }, rect.center().y), vec2(18.0, 18.0)), color);
    let galley = painter.layout_no_wrap(label.to_owned(), face(if t.bevel { 13.0 } else { 15.0 }), color);
    painter.galley(egui::pos2(rect.min.x + if t.bevel { 42.0 } else { 40.0 }, rect.center().y - galley.size().y / 2.0), galley, color);
    response
}

impl App {
    fn tick(&self) -> Duration {
        if self.busy || self.code.is_some() || self.shots.is_some() {
            Duration::from_millis(250)
        } else if self.tab == Tab::Log && self.playing {
            Duration::from_millis(500)
        } else if self.tab == Tab::Play {
            Duration::from_secs(3)
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

    fn account_line(&self) -> (String, String) {
        match &self.account {
            Some(a) => {
                let hours = |until: u64| until.saturating_sub(unix_now()) as f64 / 3600.0;
                let via = match (self.source, self.prism_expires) {
                    (Some(Source::Prism), Some(exp)) => {
                        trf("via Prism · token valid for {} h", &[&format!("{:.1}", hours(exp))])
                    }
                    (Some(Source::Own), _) => {
                        trf("via the Belumi app · token valid for {} h", &[&format!("{:.1}", hours(a.expires_at))])
                    }
                    _ => tr("signed in"),
                };
                (a.name.clone(), via)
            }
            None => match self.settings.active_saved() {
                Some(s) => (s.name.clone(), tr("saved, not renewed yet")),
                None => (tr("Not signed in"), tr("play offline or sign in under Account")),
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

    fn selected_server_label(&self) -> String {
        self.settings
            .servers
            .get(self.settings.selected_server)
            .map_or_else(|| tr("Main menu"), |s| s.name.clone())
    }

    fn handle_drops(&mut self, ctx: &egui::Context) {
        let dropped: Vec<std::path::PathBuf> = ctx.input(|i| i.raw.dropped_files.iter().map(|f| f.path().to_path_buf()).collect());
        for path in dropped {
            let is_zip = path.extension().is_some_and(|e| e.eq_ignore_ascii_case("zip"));
            let Some(name) = path.file_name().map(|n| n.to_owned()) else { continue };
            if !is_zip {
                self.status = trf("{} is not a shader pack (.zip).", &[&name.to_string_lossy()]);
                continue;
            }
            let dir = worlds::game_dir(&self.settings.game_dir).join("shaderpacks");
            let _ = std::fs::create_dir_all(&dir);
            let mut target = dir.join(&name);
            if std::fs::canonicalize(&path).ok().zip(std::fs::canonicalize(&target).ok()).is_some_and(|(a, b)| a == b) {
                self.status = trf("Added the shader pack {}.", &[&name.to_string_lossy()]);
                continue;
            }
            let mut n = 2;
            while target.exists() {
                let stem = path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
                target = dir.join(format!("{stem} ({n}).zip"));
                n += 1;
            }
            self.status = match std::fs::copy(&path, &target) {
                Ok(_) => trf("Added the shader pack {}.", &[&name.to_string_lossy()]),
                Err(e) => format!("{e}"),
            };
        }
    }

    fn banners(&mut self, ui: &mut egui::Ui) {
        if self.solas.is_none() && (!self.settings.solas_asked || self.shaders_busy) {
            card(ui, |ui| {
                ui.horizontal(|ui| {
                    crate::icons::icon(ui, Icon::Download, 18.0, t().accent);
                    ui.vertical(|ui| {
                        ui.label(RichText::new(tr("Belumi looks best with the Solas shader pack")).strong());
                        ui.label(muted(tr(
                            "Solas is made by Septonious (all rights reserved). It is downloaded from Modrinth, the author's own page, and is not part of Belumi.",
                        )));
                    });
                });
                ui.horizontal_wrapped(|ui| {
                    if self.shaders_busy {
                        ui.add(egui::ProgressBar::new(0.0).animate(true).desired_width(180.0));
                    } else {
                        if action(ui, Kind::Primary, Some(Icon::Download), &tr("Download Solas"), true, vec2(0.0, 32.0)).clicked() {
                            self.download_solas();
                        }
                        if action(ui, Kind::Secondary, None, &tr("Not now"), true, vec2(0.0, 32.0)).clicked() {
                            self.settings.solas_asked = true;
                            let _ = self.settings.save(&self.settings_path);
                        }
                        ui.hyperlink_to(tr("Open the Modrinth page"), crate::shaders::PAGE);
                    }
                });
            });
        }
        if let Some(release) = self.update.clone() {
            card(ui, |ui| {
                ui.horizontal(|ui| {
                    crate::icons::icon(ui, Icon::Download, 18.0, t().accent);
                    if let Some(tag) = self.update_done.clone() {
                        ui.label(RichText::new(trf("Updated to {}", &[&tag])).strong());
                        if action(ui, Kind::Primary, None, &tr("Restart now"), true, vec2(0.0, 32.0)).clicked() {
                            crate::update::restart();
                        }
                    } else if self.updating {
                        ui.label(RichText::new(trf("Downloading {}...", &[&release.tag])).strong());
                        let bar = match self.update_progress.percent() {
                            Some(p) => egui::ProgressBar::new(p as f32 / 100.0).text(format!("{p} %")),
                            None => egui::ProgressBar::new(0.0).animate(true),
                        };
                        ui.add(bar.desired_width(180.0));
                        ui.ctx().request_repaint_after(std::time::Duration::from_millis(100));
                    } else {
                        ui.label(RichText::new(trf("Update available: {}", &[&release.tag])).strong());
                        if action(ui, Kind::Primary, Some(Icon::Download), &tr("Update now"), true, vec2(0.0, 32.0)).clicked() {
                            self.start_update();
                        }
                        ui.hyperlink_to(tr("Open the release page"), &release.url);
                    }
                });
                if let Some(error) = &self.update_error {
                    ui.label(RichText::new(error.clone()).color(t().danger));
                }
            });
        }
        if let Some(report) = self.crash.clone() {
            card(ui, |ui| {
                ui.horizontal(|ui| {
                    crate::icons::icon(ui, Icon::Warning, 20.0, t().danger);
                    ui.vertical(|ui| {
                        ui.label(RichText::new(tr("The game stopped unexpectedly.")).strong().color(t().danger));
                        if !report.last_line.is_empty() {
                            ui.label(muted(report.last_line.clone()));
                        }
                    });
                });
                ui.horizontal_wrapped(|ui| {
                    if let Some(path) = &report.report {
                        if action(ui, Kind::Secondary, Some(Icon::Folder), &tr("Open folder"), true, vec2(0.0, 32.0)).clicked() {
                            crate::os::open(path.parent().unwrap_or(path));
                        }
                        if action(ui, Kind::Secondary, None, &tr("Copy report"), true, vec2(0.0, 32.0)).clicked() {
                            let text = std::fs::read_to_string(path).unwrap_or_default();
                            ui.ctx().copy_text(text);
                            self.status = tr("Report copied.");
                        }
                    }
                    if action(ui, Kind::Secondary, Some(Icon::Download), &tr("Pack logs for support"), true, vec2(0.0, 32.0)).clicked() {
                        let out = self.log_path.with_file_name("support-logs.zip");
                        self.status = match crate::crash::pack_for_support(&self.log_path, report.report.as_deref(), &out) {
                            Ok(()) => {
                                crate::os::open(out.parent().unwrap_or(&out));
                                trf("Saved {}", &[&out.display().to_string()])
                            }
                            Err(e) => format!("{e}"),
                        };
                    }
                });
            });
        }
        let failed: Vec<(String, String)> = crate::check::failed(&self.checks)
            .iter()
            .map(|c| (tr(c.what), c.hint.clone()))
            .collect();
        if !failed.is_empty() {
            card(ui, |ui| {
                for (what, hint) in failed {
                    ui.horizontal(|ui| {
                        crate::icons::icon(ui, Icon::Warning, 18.0, t().warn);
                        ui.vertical(|ui| {
                            ui.label(RichText::new(what).strong());
                            ui.label(muted(hint));
                        });
                    });
                }
            });
        }
    }

    fn page_play(&mut self, ui: &mut egui::Ui) {
        title(ui, &tr("Play"), &tr("Choose who plays and where"));
        self.banners(ui);
        let (name, detail) = self.account_line();
        let head = self.active_head(ui.ctx());
        let skin_key = match &self.account {
            Some(a) => a.uuid.clone(),
            None => self.settings.active_saved().map_or_else(|| "offline".to_owned(), |a| a.uuid.clone()),
        };
        self.ensure_skin(ui.ctx(), &skin_key);
        let server_label = self.selected_server_label();
        let others: Vec<(String, String)> = self
            .settings
            .accounts
            .iter()
            .filter(|a| a.uuid != self.settings.active_account)
            .map(|a| (a.name.clone(), a.uuid.clone()))
            .collect();
        let can_play = !self.playing && !self.busy && !self.diagnosing;
        let online = can_play && self.account.is_some();
        let mut switch = None;
        let mut go: Option<(bool, Target)> = None;
        let playing = self.playing;
        let offline_name = self.settings.offline_name.clone();
        let theme = t();
        if !theme.bevel {
            let at = ui.cursor().min + vec2(60.0, 60.0);
            crate::kit::glow(ui.painter(), at, 150.0, Color32::from_rgba_unmultiplied(theme.accent.r(), theme.accent.g(), theme.accent.b(), if theme.glass { 46 } else { 30 }));
        }
        let skin = self.skins.get(&skin_key);
        card(ui, |ui| {
            if let Some(skin) = skin {
                let right = ui.max_rect().right();
                let top = ui.max_rect().top();
                let stage = egui::Rect::from_min_size(egui::pos2(right - 190.0, top - 44.0), vec2(190.0, 196.0));
                crate::skin3d::paint(ui, stage, skin);
            }
            ui.horizontal(|ui| {
                avatar(ui, &name, head.as_ref(), 64.0);
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(&name).font(face(22.0)).strong());
                        if !others.is_empty() {
                            ui.menu_button(tr("Switch"), |ui| {
                                for (other, uuid) in &others {
                                    if ui.button(other).clicked() {
                                        switch = Some(uuid.clone());
                                        ui.close();
                                    }
                                }
                            });
                        }
                    });
                    ui.label(muted(detail));
                });
            });
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                let label = if playing { tr("Running...") } else { tr("Play") };
                let primary = action(ui, Kind::Primary, Some(Icon::Play), &label, online, vec2(250.0, 56.0))
                    .on_disabled_hover_text(tr("Sign in first, or play offline"));
                if primary.clicked() {
                    go = Some((true, Target::Server));
                }
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 4.0;
                    if action(ui, Kind::Ghost, None, &tr("Main menu"), online, vec2(140.0, 26.0))
                        .on_hover_text(tr("Starts the game without joining the selected server"))
                        .clicked()
                    {
                        go = Some((true, Target::MainMenu));
                    }
                    if action(ui, Kind::Ghost, None, &tr("Play offline"), can_play, vec2(140.0, 26.0))
                        .on_hover_text(trf("Play offline as {}", &[&offline_name]))
                        .clicked()
                    {
                        go = Some((false, Target::Server));
                    }
                });
            });
        });
        if let Some(uuid) = switch {
            self.switch_account(&uuid);
        }
        if let Some((online, target)) = go {
            self.play(online, target);
        }
        let version = self.settings.active_version().trim().to_owned();
        let line = if version.is_empty() {
            trf("Starts Minecraft as {} on {}", &[&name, &server_label])
        } else {
            trf("Starts Minecraft {} as {} on {}", &[&crate::versions::label(&version), &name, &server_label])
        };
        ui.label(muted(line));

        ui.add_space(6.0);
        let tabs = [tr("Servers"), tr("Worlds")];
        if let Some(i) = segmented(ui, &tabs, self.play_list) {
            self.play_list = i;
        }
        if self.play_list == 0 {
            self.server_list(ui);
        } else {
            self.world_list(ui, online || (can_play && self.account.is_none()));
        }
    }

    fn server_list(&mut self, ui: &mut egui::Ui) {
        self.probe_servers();
        let t = t();
        let mut changed = false;
        let mut remove = None;
        let accounts: Vec<(String, String)> = self.settings.accounts.iter().map(|a| (a.name.clone(), a.uuid.clone())).collect();
        let count = self.settings.servers.len();
        for i in 0..count {
            let selected = self.settings.selected_server == i;
            let (name, address) = {
                let s = &self.settings.servers[i];
                (s.name.clone(), s.address.clone())
            };
            let probe = self.probes.get(address.trim()).map(|p| p.0.clone()).unwrap_or(Probe::Unknown);
            let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), 46.0), Sense::click());
            let lift = ui.ctx().animate_value_with_time(response.id.with("row"), if response.hovered() { 1.0 } else { 0.0 }, 0.12);
            let radius = f32::from(t.control_radius);
            {
                let painter = ui.painter();
                if selected {
                    painter.rect_filled(rect, radius, t.accent_dim);
                    painter.rect_stroke(rect, radius, Stroke::new(t.border.max(1.0) + if t.bevel { 0.0 } else { 0.5 }, t.accent), egui::StrokeKind::Inside);
                } else if !t.bevel {
                    painter.rect_filled(rect, radius, Color32::from_white_alpha((7.0 + 9.0 * lift) as u8));
                } else if lift > 0.01 {
                    painter.rect_filled(rect, radius, if t.bevel { Color32::from_white_alpha((14.0 * lift) as u8) } else { Color32::from_white_alpha((16.0 * lift) as u8) });
                }
            }
            let (color, label) = match &probe {
                Probe::Up(s) => (t.ok, trf("{} of {} players", &[&s.online.to_string(), &s.max.to_string()]) + &format!(" · {} ms", s.latency_ms)),
                Probe::Down => (t.danger, tr("Offline")),
                Probe::Checking | Probe::Unknown => (t.muted, String::new()),
            };
            {
                let painter = ui.painter();
                let galley = painter.layout_no_wrap(name.clone(), face(15.0), t.text);
                painter.galley(egui::pos2(rect.min.x + 38.0, rect.min.y + 6.0), galley, t.text);
                let galley = painter.layout_no_wrap(address.clone(), face(12.0), t.muted);
                painter.galley(egui::pos2(rect.min.x + 38.0, rect.min.y + 25.0), galley, t.muted);
                if t.bevel {
                    painter.rect_filled(egui::Rect::from_center_size(egui::pos2(rect.min.x + 20.0, rect.center().y), vec2(10.0, 10.0)), 0.0, color);
                } else {
                    let lit = match &probe {
                        Probe::Up(s) => match s.latency_ms {
                            0..=60 => 4,
                            61..=120 => 3,
                            121..=250 => 2,
                            _ => 1,
                        },
                        _ => 0,
                    };
                    for b in 0..4 {
                        let h = 5.0 + 3.0 * b as f32;
                        let x = rect.min.x + 12.0 + 4.5 * b as f32;
                        let bar = egui::Rect::from_min_max(egui::pos2(x, rect.center().y + 8.0 - h), egui::pos2(x + 3.0, rect.center().y + 8.0));
                        let c = if b < lit { color } else { Color32::from_white_alpha(34) };
                        painter.rect_filled(bar, 1.0, c);
                    }
                }
                if !label.is_empty() {
                    let galley = painter.layout_no_wrap(label, face(12.5), if matches!(probe, Probe::Up(_)) { t.text } else { t.muted });
                    let x = rect.max.x - 40.0 - galley.size().x;
                    painter.galley(egui::pos2(x, rect.center().y - galley.size().y / 2.0), galley, t.text);
                }
            }
            let close = egui::Rect::from_center_size(egui::pos2(rect.max.x - 20.0, rect.center().y), vec2(26.0, 26.0));
            let close_response = ui.interact(close, response.id.with("close"), Sense::click());
            let c = if close_response.hovered() { t.danger } else { Color32::from_rgba_unmultiplied(t.muted.r(), t.muted.g(), t.muted.b(), 130) };
            crate::icons::paint(ui.painter(), Icon::Close, egui::Rect::from_center_size(close.center(), vec2(14.0, 14.0)), c);
            if close_response.clicked() {
                remove = Some(i);
            } else if response.clicked() {
                self.settings.selected_server = i;
                changed = true;
            }
            if selected {
                ui.horizontal(|ui| {
                    ui.add_space(38.0);
                    ui.label(muted(tr("Version")));
                    let server = &mut self.settings.servers[i];
                    egui::ComboBox::from_id_salt(("version", i))
                        .selected_text(crate::versions::label(&server.version))
                        .width(150.0)
                        .show_ui(ui, |ui| {
                            let mut pick = |ui: &mut egui::Ui, id: &str| {
                                let on = server.version.trim() == id
                                    || (id == crate::versions::AUTO && crate::versions::find(&server.version).is_none());
                                if ui.selectable_label(on, crate::versions::label(id)).clicked() {
                                    server.version = if id == crate::versions::AUTO { String::new() } else { id.to_owned() };
                                    changed = true;
                                }
                            };
                            pick(ui, crate::versions::AUTO);
                            for v in crate::versions::VERSIONS {
                                pick(ui, v.id);
                            }
                        });
                    if accounts.len() > 1 {
                        ui.label(muted(tr("Account")));
                        let current = accounts
                            .iter()
                            .find(|(_, u)| *u == server.account)
                            .map_or_else(|| tr("Active account"), |(n, _)| n.clone());
                        egui::ComboBox::from_id_salt(("server-account", i))
                            .selected_text(current)
                            .width(150.0)
                            .show_ui(ui, |ui| {
                                if ui.selectable_label(server.account.is_empty(), tr("Active account")).clicked() {
                                    server.account.clear();
                                    changed = true;
                                }
                                for (n, u) in &accounts {
                                    if ui.selectable_label(server.account == *u, n).clicked() {
                                        server.account = u.clone();
                                        changed = true;
                                    }
                                }
                            });
                    }
                });
            }
        }
        if count == 0 {
            empty_state(ui, Icon::Server, &tr("No server yet: the game starts in its main menu."));
        }
        if let Some(i) = remove {
            self.settings.servers.remove(i);
            if self.settings.selected_server >= i && self.settings.selected_server > 0 {
                self.settings.selected_server -= 1;
            }
            changed = true;
        }
        if self.add_server {
            card(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.add(egui::TextEdit::singleline(&mut self.new_server.name).hint_text(tr("Name")).desired_width(120.0));
                    ui.add(egui::TextEdit::singleline(&mut self.new_server.address).hint_text("host:port").desired_width(200.0));
                    let ready = !self.new_server.address.trim().is_empty();
                    if action(ui, Kind::Secondary, Some(Icon::Plus), &tr("Add"), ready, vec2(0.0, 30.0)).clicked() {
                        let mut entry = std::mem::take(&mut self.new_server);
                        entry.address = entry.address.trim().to_owned();
                        if entry.name.trim().is_empty() {
                            entry.name = entry.address.clone();
                        }
                        self.settings.servers.push(entry);
                        self.settings.selected_server = self.settings.servers.len() - 1;
                        self.add_server = false;
                        changed = true;
                    }
                    if action(ui, Kind::Ghost, None, &tr("Cancel"), true, vec2(0.0, 30.0)).clicked() {
                        self.add_server = false;
                    }
                });
            });
        } else if action(ui, Kind::Ghost, Some(Icon::Plus), &tr("Add server"), true, vec2(0.0, 28.0)).clicked() {
            self.add_server = true;
        }
        if changed {
            self.save();
        }
    }

    fn world_list(&mut self, ui: &mut egui::Ui, can_start: bool) {
        self.refresh_worlds();
        let t = t();
        let now = std::time::SystemTime::now();
        let list = self.worlds.clone();
        let online = self.account.is_some();
        if list.is_empty() {
            let dir = worlds::game_dir(&self.settings.game_dir).join("saves");
            empty_state(ui, Icon::World, &trf("No worlds found in {}", &[&dir.display().to_string()]));
            return;
        }
        for world in list {
            let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), 46.0), Sense::hover());
            let lift = ui.ctx().animate_value_with_time(response.id.with("w"), if response.hovered() { 1.0 } else { 0.0 }, 0.12);
            if lift > 0.01 {
                ui.painter().rect_filled(rect, f32::from(t.control_radius), Color32::from_white_alpha((16.0 * lift) as u8));
            }
            crate::icons::paint(ui.painter(), Icon::World, egui::Rect::from_center_size(egui::pos2(rect.min.x + 20.0, rect.center().y), vec2(18.0, 18.0)), t.muted);
            let galley = ui.painter().layout_no_wrap(world.folder.clone(), face(15.0), t.text);
            ui.painter().galley(egui::pos2(rect.min.x + 38.0, rect.min.y + 6.0), galley, t.text);
            if let Some(played) = world.played {
                let text = trf("Last played {}", &[&worlds::ago(played, now)]);
                let galley = ui.painter().layout_no_wrap(text, face(12.0), t.muted);
                ui.painter().galley(egui::pos2(rect.min.x + 38.0, rect.min.y + 25.0), galley, t.muted);
            }
            let button = egui::Rect::from_min_size(egui::pos2(rect.max.x - 110.0, rect.min.y + 8.0), vec2(104.0, 30.0));
            let clicked = ui
                .scope_builder(egui::UiBuilder::new().id_salt(("world", &world.folder)).max_rect(button), |inner| {
                    action(inner, Kind::Secondary, Some(Icon::Play), &tr("Start"), can_start, vec2(104.0, 30.0))
                        .on_hover_text(tr("Start in this world"))
                        .clicked()
                })
                .inner;
            if clicked {
                self.play(online, Target::World(world.folder.clone()));
            }
        }
    }

    fn page_account(&mut self, ui: &mut egui::Ui) {
        title(ui, &tr("Account"), &tr("Who plays, and how the sign-in is kept"));
        let (name, detail) = self.account_line();
        let head = self.active_head(ui.ctx());
        card(ui, |ui| {
            ui.horizontal(|ui| {
                avatar(ui, &name, head.as_ref(), 56.0);
                ui.vertical(|ui| {
                    ui.label(RichText::new(&name).font(face(20.0)).strong());
                    ui.label(muted(detail));
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let can = !self.busy && self.settings.active_saved().is_some();
                    if action(ui, Kind::Ghost, Some(Icon::Refresh), &tr("Renew now"), can, vec2(0.0, 30.0))
                        .on_hover_text(tr("Gets a fresh token for the active account"))
                        .clicked()
                    {
                        self.renew();
                    }
                });
            });
        });
        if let Some(code) = &self.code {
            well(ui, |ui| {
                ui.label(trf("Open {} and enter:", &[&code.verification_uri]));
                ui.label(RichText::new(&code.user_code).font(egui::FontId::monospace(30.0)).strong().color(t().accent));
            });
        }
        ui.horizontal_wrapped(|ui| {
            if action(ui, Kind::Secondary, Some(Icon::Plus), &tr("Add account"), !self.busy, vec2(0.0, 34.0))
                .on_hover_text(tr("Signs in another Microsoft account; the others stay saved"))
                .clicked()
            {
                self.sign_in();
            }
            if self.busy && action(ui, Kind::Ghost, None, &tr("Cancel"), true, vec2(0.0, 34.0)).clicked() {
                self.cancel.store(true, std::sync::atomic::Ordering::Relaxed);
                self.epoch.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                self.busy = false;
                self.code = None;
                self.status = tr("Cancelled.");
            }
            if action(ui, Kind::Secondary, None, &tr("Use my Prism account"), !self.busy, vec2(0.0, 34.0)).clicked() {
                self.settings.use_prism = true;
                self.import_prism(false);
                self.save();
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if action(ui, Kind::Danger, None, &tr("Sign out"), !self.busy, vec2(0.0, 34.0)).clicked() {
                    self.sign_out();
                }
            });
        });
        let others: Vec<(String, String)> = self
            .settings
            .accounts
            .iter()
            .filter(|a| a.uuid != self.settings.active_account || self.source == Some(Source::Prism))
            .map(|a| (a.name.clone(), a.uuid.clone()))
            .collect();
        section(ui, &tr("Other accounts"));
        if others.is_empty() {
            ui.label(muted(tr("No other accounts yet.")));
        }
        let (mut switch, mut remove) = (None, None);
        for (name, uuid) in others {
            let head = self.head_texture(ui.ctx(), &uuid);
            card(ui, |ui| {
                ui.horizontal(|ui| {
                    avatar(ui, &name, head.as_ref(), 36.0);
                    ui.label(RichText::new(&name).strong());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if action(ui, Kind::Danger, None, &tr("Remove"), !self.busy, vec2(0.0, 30.0)).clicked() {
                            remove = Some(uuid.clone());
                        }
                        if action(ui, Kind::Secondary, None, &tr("Use"), !self.busy, vec2(0.0, 30.0)).clicked() {
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
        ui.add_space(4.0);
        if ui
            .checkbox(&mut self.settings.use_prism, tr("Keep using Prism's login automatically"))
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
        ui.label(muted(tr("The sign-in is kept in your desktop's keyring when there is one (secret-tool), otherwise in an owner-only file.")));
    }

    fn page_log(&mut self, ui: &mut egui::Ui) {
        title(ui, &tr("Game log"), &tr("What the game wrote the last time it ran"));
        ui.horizontal_wrapped(|ui| {
            if action(ui, Kind::Secondary, Some(Icon::Refresh), &tr("Refresh"), true, vec2(0.0, 30.0)).clicked() {
                self.log_read = None;
            }
            if action(ui, Kind::Secondary, None, &tr("Copy"), !self.log_text.is_empty(), vec2(0.0, 30.0)).clicked() {
                ui.ctx().copy_text(self.log_text.clone());
                self.status = tr("Log copied.");
            }
            if action(ui, Kind::Secondary, Some(Icon::Folder), &tr("Open folder"), true, vec2(0.0, 30.0)).clicked() {
                crate::os::open(self.log_path.parent().unwrap_or(&self.log_path));
            }
            if action(ui, Kind::Secondary, Some(Icon::Shield), &tr("Run diagnose"), !self.diagnosing && !self.playing, vec2(0.0, 30.0))
                .on_hover_text(tr("Runs benchmarks and screenshots in a scratch folder and collects a report to send for support."))
                .clicked()
            {
                self.start_diagnose();
            }
        });
        self.read_log();
        well(ui, |ui| {
            if self.log_text.is_empty() {
                empty_state(ui, Icon::Log, &trf("Nothing yet. The game writes to {}", &[&self.log_path.display().to_string()]));
            } else {
                egui::ScrollArea::both().stick_to_bottom(true).max_height(360.0).auto_shrink([false, true]).show(ui, |ui| {
                    ui.add(egui::Label::new(RichText::new(&self.log_text).font(egui::FontId::monospace(12.5)).color(t().muted)).extend());
                });
            }
        });
    }

    fn page_settings(&mut self, ui: &mut egui::Ui) {
        title(ui, &tr("Settings"), &tr("Look, game files and how the game starts"));
        let mut changed = false;
        section(ui, &tr("Look"));
        ui.horizontal_wrapped(|ui| {
            let labels: Vec<String> = Style::ALL.iter().map(|s| s.label().to_owned()).collect();
            let at = Style::ALL.iter().position(|s| s.id() == self.settings.style).unwrap_or(0);
            if let Some(i) = segmented(ui, &labels, at) {
                self.settings.style = Style::ALL[i].id().to_owned();
                changed = true;
            }
            let langs = ["Auto".to_owned(), "English".to_owned(), "Deutsch".to_owned()];
            let at = match self.settings.language.as_str() {
                "en" => 1,
                "de" => 2,
                _ => 0,
            };
            if let Some(i) = segmented(ui, &langs, at) {
                self.settings.language = ["auto", "en", "de"][i].to_owned();
                changed = true;
            }
        });
        section(ui, &tr("Game files"));
        card(ui, |ui| {
            egui::Grid::new("files").num_columns(2).spacing([14.0, 10.0]).show(ui, |ui| {
                let typed = &mut self.fields_typed;
                let mut field = |ui: &mut egui::Ui, label: &str, value: &mut String| {
                    ui.label(muted(tr(label)));
                    let response = ui.add(egui::TextEdit::singleline(value).desired_width(ui.available_width().max(260.0)));
                    *typed |= response.changed();
                    if response.lost_focus() && *typed {
                        *typed = false;
                        changed = true;
                    }
                    response.on_hover_text(value.clone());
                    ui.end_row();
                };
                field(ui, "Azure client ID", &mut self.settings.client_id);
                field(ui, "Game binary", &mut self.settings.client_binary);
                field(ui, "Resources", &mut self.settings.resources);
                field(ui, "Game folder", &mut self.settings.game_dir);
                field(ui, "Offline name", &mut self.settings.offline_name);
                field(ui, "Prism accounts.json", &mut self.settings.prism_accounts_path);
            });
        });
        section(ui, &tr("Launch options"));
        card(ui, |ui| {
            changed |= ui.checkbox(&mut self.settings.fullscreen, tr("Fullscreen")).changed();
            ui.horizontal(|ui| {
                ui.label(muted(tr("Window size")));
                let mut w = self.settings.window_width.to_string();
                let mut h = self.settings.window_height.to_string();
                let a = ui.add_enabled(!self.settings.fullscreen, egui::TextEdit::singleline(&mut w).desired_width(56.0).hint_text("auto"));
                ui.label("×");
                let b = ui.add_enabled(!self.settings.fullscreen, egui::TextEdit::singleline(&mut h).desired_width(56.0).hint_text("auto"));
                if a.changed() || b.changed() {
                    self.settings.window_width = w.trim().parse().unwrap_or(0);
                    self.settings.window_height = h.trim().parse().unwrap_or(0);
                    changed = true;
                }
            });
            ui.horizontal(|ui| {
                ui.label(muted(tr("Graphics API")));
                let opts = [tr("Auto"), "Vulkan".to_owned(), "DirectX 12".to_owned()];
                let at = match self.settings.graphics_api.as_str() {
                    "vulkan" => 1,
                    "dx12" => 2,
                    _ => 0,
                };
                if let Some(i) = segmented(ui, &opts, at) {
                    self.settings.graphics_api = ["auto", "vulkan", "dx12"][i].to_owned();
                    changed = true;
                }
            });
            ui.horizontal(|ui| {
                ui.label(muted(tr("After the game starts")));
                let opts = [tr("Keep open"), tr("Minimize"), tr("Close")];
                let at = match self.settings.after_start.as_str() {
                    "minimize" => 1,
                    "close" => 2,
                    _ => 0,
                };
                if let Some(i) = segmented(ui, &opts, at) {
                    self.settings.after_start = ["keep", "minimize", "close"][i].to_owned();
                    changed = true;
                }
            });
        });
        section(ui, &tr("Shader packs"));
        card(ui, |ui| {
            let dir = worlds::game_dir(&self.settings.game_dir).join("shaderpacks");
            let packs: Vec<String> = std::fs::read_dir(&dir)
                .into_iter()
                .flatten()
                .flatten()
                .filter_map(|e| e.file_name().into_string().ok())
                .filter(|n| n.to_lowercase().ends_with(".zip"))
                .collect();
            if packs.is_empty() {
                ui.label(muted(tr("None")));
            }
            for pack in packs {
                ui.horizontal(|ui| {
                    crate::icons::icon(ui, Icon::Cube, 16.0, t().muted);
                    ui.label(pack);
                });
            }
            ui.label(muted(tr("Drop a shader pack (.zip) here to add it")));
        });
        section(ui, &tr("Shaders"));
        card(ui, |ui| {
            match self.solas.clone() {
                Some(file) => {
                    ui.label(trf("Solas is installed: {}", &[&file]));
                }
                None => {
                    ui.horizontal(|ui| {
                        if action(ui, Kind::Secondary, Some(Icon::Download), &tr("Download Solas"), !self.shaders_busy, vec2(0.0, 28.0)).clicked() {
                            self.download_solas();
                        }
                        ui.hyperlink_to(tr("Open the Modrinth page"), crate::shaders::PAGE);
                    });
                }
            }
            ui.label(muted(tr(
                "Solas by Septonious, all rights reserved. Belumi downloads it from Modrinth and never ships it. The game picks it by itself the first time it finds it.",
            )));
        });
        section(ui, &tr("Client plugins"));
        card(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(tr("List address"));
                changed |= ui
                    .add(egui::TextEdit::singleline(&mut self.settings.plugin_index_url).hint_text("https://.../plugins.json").desired_width(ui.available_width().max(260.0) - 90.0))
                    .changed();
                if action(ui, Kind::Secondary, None, &tr("Refresh"), !self.plugin_busy && !self.settings.plugin_index_url.trim().is_empty(), vec2(0.0, 28.0)).clicked() {
                    self.refresh_plugin_list();
                }
            });
            let have = crate::plugins::installed(&self.plugin_dir());
            if have.is_empty() && self.plugin_list.is_empty() {
                ui.label(muted(tr("No plugins yet. Enter the address of a plugin list, or put a plugin in the game folder's belumi-plugins.")));
            }
            for p in &have {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(format!("{} {}", p.name, p.version)).strong());
                    let label = if p.enabled { tr("Switch off") } else { tr("Switch on") };
                    if action(ui, Kind::Secondary, None, &label, true, vec2(0.0, 24.0)).clicked() {
                        self.switch_plugin(&p.name, !p.enabled);
                    }
                    if action(ui, Kind::Secondary, None, &tr("Remove"), true, vec2(0.0, 24.0)).clicked() {
                        self.remove_plugin(&p.name);
                    }
                });
            }
            let mut to_install = None;
            for e in &self.plugin_list {
                let mine = have.iter().find(|p| p.name == e.name);
                if mine.is_some_and(|p| p.version == e.version) {
                    continue;
                }
                ui.separator();
                ui.horizontal(|ui| {
                    ui.label(RichText::new(format!("{} {}", e.name, e.version)).strong());
                    let label = if mine.is_some() { tr("Update") } else { tr("Install") };
                    if action(ui, Kind::Primary, Some(Icon::Download), &label, !self.plugin_busy, vec2(0.0, 24.0)).clicked() {
                        to_install = Some(e.clone());
                    }
                });
                if !e.description.is_empty() {
                    ui.label(muted(e.description.clone()));
                }
                let asks = if e.permissions.is_empty() { tr("nothing") } else { e.permissions.join(", ") };
                ui.label(muted(trf("Asks for: {}", &[&asks])));
            }
            if let Some(entry) = to_install {
                self.install_plugin(entry);
            }
            ui.label(muted(tr("A plugin is code that runs in a sandbox and may only do what it asks for. Install plugins of people you trust.")));
        });
        section(ui, &tr("Discord"));
        card(ui, |ui| {
            changed |= ui
                .checkbox(&mut self.settings.discord_presence, tr("Show Belumi on my Discord profile"))
                .on_hover_text(tr("Applies to the launcher and to the game."))
                .changed();
            changed |= ui
                .checkbox(&mut self.settings.discord_show_account, tr("Show my account name in the launcher"))
                .changed();
            ui.label(muted(tr(
                "What the game shows (server, world, time) is set in the game: Right Shift, Modules, Discord presence.",
            )));
        });
        section(ui, &tr("Updates"));
        card(ui, |ui| {
            ui.horizontal(|ui| {
                changed |= ui
                    .checkbox(&mut self.settings.check_updates, tr("Check for updates at start"))
                    .on_hover_text(tr("Asks GitHub's public release list; nothing is installed automatically."))
                    .changed();
                if action(ui, Kind::Secondary, None, &tr("Check now"), !self.updating, vec2(0.0, 28.0)).clicked() {
                    self.update_announced = false;
                    self.update_manual = true;
                    self.update_done = None;
                    self.check_for_update(true);
                    self.status = tr("Looking for updates...");
                }
                let channel = crate::update::Channel::parse(&self.settings.update_channel);
                egui::ComboBox::from_id_salt("channel").selected_text(channel.name()).show_ui(ui, |ui| {
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
            self.checks = crate::check::run(&self.settings);
        }
    }
}

pub(crate) fn tail(path: &std::path::Path, bytes: u64) -> String {
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
        text.split_once('\n').map_or(String::new(), |(_, rest)| rest.to_owned())
    } else {
        text
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        if let Ok(mut ctx) = self.tx.ctx.lock()
            && ctx.is_none()
        {
            *ctx = Some(ui.ctx().clone());
        }
        let language = i18n::Lang::parse(&self.settings.language, &i18n::system_language());
        i18n::set(language);
        let want = theme::Style::parse(&self.settings.style);
        if !self.styled || theme::current() != want {
            theme::set(want);
            apply_theme(ui.ctx());
            self.styled = true;
        }
        self.update_presence();
        self.refresh_solas();
        if self.checks.is_empty() {
            self.checks = crate::check::run(&self.settings);
        }
        paint_ground(ui);
        self.shot_step(ui.ctx());
        self.poll();
        self.watch_prism();
        self.watch_renewal();
        self.handle_drops(ui.ctx());
        self.handle_join(ui.ctx());
        if let Some(command) = self.window_cmd.take() {
            ui.ctx().send_viewport_cmd(command);
        }
        ui.ctx().request_repaint_after(self.tick());
        let theme = t();

        egui::Panel::left("nav")
            .exact_size(if theme.glass { 186.0 } else { 176.0 })
            .frame(egui::Frame::new().fill(if theme.bevel { Color32::TRANSPARENT } else { theme.nav }).inner_margin(egui::Margin::same(14)))
            .show(ui, |ui| {
                let logo = self.logo.get_or_insert_with(|| {
                    let icon = eframe::icon_data::from_png_bytes(include_bytes!("../assets/logo-256.png")).unwrap_or_default();
                    let image = egui::ColorImage::from_rgba_unmultiplied([icon.width as usize, icon.height as usize], &icon.rgba);
                    ui.ctx().load_texture("logo", image, egui::TextureOptions::LINEAR)
                });
                if !theme.bevel {
                    let at = ui.cursor().min + vec2(28.0, 28.0);
                    crate::kit::glow(ui.painter(), at, 70.0, Color32::from_rgba_unmultiplied(theme.accent.r(), theme.accent.g(), theme.accent.b(), 60));
                }
                ui.add(egui::Image::new(&*logo).fit_to_exact_size(vec2(56.0, 56.0)));
                ui.label(RichText::new("Belumi").font(face(22.0)).strong());
                ui.add_space(10.0);
                let first = ui.cursor().min;
                let index = Tab::ALL.iter().position(|(tab, _, _)| *tab == self.tab).unwrap_or(0) as f32;
                let slide = ui.ctx().animate_value_with_time(egui::Id::NULL.with("nav-slide"), index, 0.16);
                if !theme.bevel {
                    let pitch = 40.0 + ui.spacing().item_spacing.y - 4.0;
                    let pill = egui::Rect::from_min_size(first + vec2(0.0, slide * pitch), vec2(ui.available_width(), 40.0));
                    let radius = f32::from(theme.control_radius);
                    ui.painter().rect_filled(pill, radius, theme.accent_dim);
                    if theme.glass {
                        ui.painter().rect_stroke(pill, radius, Stroke::new(1.0, Color32::from_white_alpha(36)), egui::StrokeKind::Inside);
                    }
                    ui.painter().rect_filled(egui::Rect::from_min_size(pill.min + vec2(0.0, 9.0), vec2(3.0, 22.0)), 1.5, theme.accent);
                }
                for (tab, icon, label) in Tab::ALL {
                    if nav_item(ui, icon, &tr(label), self.tab == tab).clicked() {
                        self.tab = tab;
                        self.log_read = None;
                    }
                    ui.add_space(-4.0);
                }
            });

        egui::Panel::bottom("status")
            .exact_size(if theme.bevel { 40.0 } else { 34.0 })
            .frame(egui::Frame::new().fill(if theme.bevel { Color32::TRANSPARENT } else { theme.status_bar }).inner_margin(egui::Margin::symmetric(16, 7)))
            .show(ui, |ui| {
                if theme.bevel {
                    paint_ledge(ui.painter(), ui.max_rect().expand2(vec2(16.0, 7.0)));
                }
                ui.horizontal_centered(|ui| {
                    let (word, color) = if self.playing {
                        (tr("Running"), theme.ok)
                    } else if self.busy {
                        (tr("Working"), theme.warn)
                    } else {
                        (tr("Idle"), theme.muted)
                    };
                    dot(ui, color, 8.0);
                    ui.label(RichText::new(word).color(theme.text).size(12.5));
                    ui.label(muted(&self.status));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(muted(format!("Belumi {}  \u{b7}  Minecraft 26.2 / 26.3", crate::VERSION)));
                    });
                });
            });

        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(Color32::TRANSPARENT).inner_margin(egui::Margin::symmetric(22, 18)))
            .show(ui, |ui| {
                if theme.bevel {
                    paint_board(ui.painter(), ui.max_rect().expand2(vec2(16.0, 12.0)), ui.ctx());
                }
                egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| match self.tab {
                    Tab::Play => self.page_play(ui),
                    Tab::Account => self.page_account(ui),
                    Tab::Log => self.page_log(ui),
                    Tab::Settings => self.page_settings(ui),
                });
            });
        let _ = CornerRadius::ZERO;
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
        std::fs::write(&path, "alpha
beta
gamma
").unwrap();
        assert_eq!(tail(&path, 100), "alpha
beta
gamma
");
        assert_eq!(tail(&path, 9), "gamma
");
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
            account: String::new(),
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
        self.checks = vec![crate::check::Check { what: "Game file", ok: true, hint: String::new() }];
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
        if let Ok(lang) = std::env::var("BELUMI_LAUNCHER_SHOT_LANG") {
            self.settings.language = lang;
        }
        self.tab = tab;
        shots.frames += 1;
        if shots.frames == 30 && !shots.waiting {
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
