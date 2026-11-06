use crate::fonts;
use crate::icons::{self, Icon};
use crate::theme::{self, Theme};
use eframe::egui::{self, Color32, CornerRadius, FontId, Pos2, Rect, RichText, Sense, Shape, Stroke, Vec2, vec2};

pub fn t() -> Theme {
    theme::theme(theme::current())
}

/// The face the whole window is set in: Inter, or the pixel face in the Pixel look.
pub fn face(size: f32) -> FontId {
    if t().bevel { fonts::pixel_font(size) } else { fonts::ui_font(size) }
}

pub fn apply_theme(ctx: &egui::Context) {
    fonts::install(ctx);
    let t = t();
    let mut style = (*ctx.global_style()).clone();
    use egui::TextStyle::*;
    let scale = if t.bevel { 1.1 } else { 1.0 };
    style.text_styles.insert(Body, face(14.0 * scale));
    style.text_styles.insert(Button, face(14.0 * scale));
    style.text_styles.insert(Small, face(12.0 * scale));
    style.text_styles.insert(Heading, face(22.0 * scale));
    style.text_styles.insert(Monospace, FontId::monospace(12.5));
    let v = &mut style.visuals;
    v.dark_mode = true;
    v.override_text_color = Some(t.text);
    v.panel_fill = Color32::TRANSPARENT;
    v.window_fill = t.panel;
    v.extreme_bg_color = if t.bevel {
        Color32::from_rgb(0xff, 0xf8, 0xe0)
    } else if t.glass {
        Color32::from_rgba_unmultiplied(0x0f, 0x0c, 0x20, 150)
    } else {
        Color32::from_black_alpha(90)
    };
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
    v.widgets.inactive.bg_stroke = if t.bevel || t.glass { Stroke::new(t.border, t.line) } else { Stroke::NONE };
    v.widgets.hovered.bg_fill = t.control_hover;
    v.widgets.hovered.weak_bg_fill = t.control_hover;
    v.widgets.hovered.bg_stroke = Stroke::new(t.border.max(1.0), t.accent);
    v.widgets.active.bg_fill = t.accent_dim;
    v.widgets.active.weak_bg_fill = t.accent_dim;
    v.widgets.active.fg_stroke = Stroke::new(1.0, t.text);
    v.widgets.active.bg_stroke = Stroke::new(t.border.max(1.0), t.accent);
    style.spacing.item_spacing = vec2(10.0, 10.0);
    style.spacing.button_padding = t.button_padding;
    style.spacing.interact_size.y = 30.0;
    ctx.set_global_style(style);
}

fn hash(x: i32, y: i32) -> u32 {
    (x.wrapping_mul(73_856_093) ^ y.wrapping_mul(19_349_663)) as u32
}

fn radial(painter: &egui::Painter, centre: Pos2, radius: f32, color: Color32) {
    const SEGMENTS: usize = 28;
    let mut mesh = egui::Mesh::default();
    mesh.colored_vertex(centre, color);
    let rim = Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 0);
    for i in 0..SEGMENTS {
        let a = i as f32 / SEGMENTS as f32 * std::f32::consts::TAU;
        mesh.colored_vertex(centre + vec2(a.cos(), a.sin()) * radius, rim);
    }
    for i in 0..SEGMENTS {
        mesh.add_triangle(0, 1 + i as u32, 1 + ((i + 1) % SEGMENTS) as u32);
    }
    painter.add(Shape::mesh(mesh));
}

/// A small pixel-art texture, built once per window and sampled without smoothing.
fn pixel_texture(ctx: &egui::Context, key: &str, size: [usize; 2], texel: impl Fn(usize, usize) -> Color32) -> egui::TextureHandle {
    let id = egui::Id::NULL.with("belumi-texture").with(key);
    if let Some(handle) = ctx.data(|d| d.get_temp::<egui::TextureHandle>(id)) {
        return handle;
    }
    let mut pixels = Vec::with_capacity(size[0] * size[1]);
    for y in 0..size[1] {
        for x in 0..size[0] {
            pixels.push(texel(x, y));
        }
    }
    let image = egui::ColorImage::new(size, pixels);
    let handle = ctx.load_texture(key, image, egui::TextureOptions::NEAREST_REPEAT);
    ctx.data_mut(|d| d.insert_temp(id, handle.clone()));
    handle
}

/// Tiles `texture` over `rect`, every texel drawn `scale` points wide.
fn tile(painter: &egui::Painter, rect: Rect, texture: &egui::TextureHandle, scale: f32) {
    let size = texture.size_vec2() * scale;
    let uv = Rect::from_min_size(Pos2::ZERO, rect.size() / size);
    painter.image(texture.id(), rect, uv, Color32::WHITE);
}

/// Parchment, 16 texels square: one flat tone with a few slightly darker flecks.
fn paper_texture(ctx: &egui::Context) -> egui::TextureHandle {
    pixel_texture(ctx, "paper", [16, 16], |x, y| {
        let base = t().bg;
        if hash(x as i32, y as i32) % 11 == 0 {
            Color32::from_rgb(base.r().saturating_sub(10), base.g().saturating_sub(14), base.b().saturating_sub(20))
        } else {
            base
        }
    })
}

/// The wooden table the window sits on: flat boards, dark seams, a few grain flecks.
fn wood_texture(ctx: &egui::Context) -> egui::TextureHandle {
    pixel_texture(ctx, "wood", [32, 12], |x, y| {
        let base = t().nav;
        let board = hash(0, 77) % 3;
        if y % 6 == 5 {
            return Color32::from_rgb(base.r() - 0x10, base.g() - 0x18, base.b() - 0x14);
        }
        let fleck = hash((x / 3) as i32, (y / 6) as i32 + board as i32) % 9;
        if fleck == 0 && y % 3 == 1 {
            Color32::from_rgb(base.r() - 0x0c, base.g() - 0x16, base.b() - 0x14)
        } else {
            base
        }
    })
}

/// The window's ground: flat, a gradient with two soft glows (glass), or a wooden table (Pixel).
pub fn paint_ground(ui: &egui::Ui) {
    let t = t();
    let rect = ui.ctx().content_rect();
    let painter = ui.painter();
    if t.glass {
        let mut mesh = egui::Mesh::default();
        mesh.colored_vertex(rect.left_top(), Color32::from_rgb(0x1a, 0x13, 0x3a));
        mesh.colored_vertex(rect.right_top(), Color32::from_rgb(0x12, 0x18, 0x34));
        mesh.colored_vertex(rect.right_bottom(), Color32::from_rgb(0x08, 0x10, 0x1e));
        mesh.colored_vertex(rect.left_bottom(), Color32::from_rgb(0x0e, 0x14, 0x2a));
        mesh.add_triangle(0, 1, 2);
        mesh.add_triangle(0, 2, 3);
        painter.add(Shape::mesh(mesh));
        radial(painter, rect.left_top() + vec2(260.0, 40.0), 340.0, Color32::from_rgba_unmultiplied(0x7a, 0x4d, 0xff, 70));
        radial(painter, rect.right_bottom() - vec2(120.0, 40.0), 300.0, Color32::from_rgba_unmultiplied(0x2b, 0x6b, 0xff, 46));
    } else if t.bevel {
        tile(painter, rect, &wood_texture(ui.ctx()), 4.0);
    } else {
        painter.rect_filled(rect, 0.0, t.bg);
    }
}

fn ring(painter: &egui::Painter, rect: Rect, w: f32, color: Color32) {
    painter.rect_filled(Rect::from_min_size(rect.min, vec2(rect.width(), w)), 0.0, color);
    painter.rect_filled(Rect::from_min_size(egui::pos2(rect.min.x, rect.max.y - w), vec2(rect.width(), w)), 0.0, color);
    painter.rect_filled(Rect::from_min_size(rect.min, vec2(w, rect.height())), 0.0, color);
    painter.rect_filled(Rect::from_min_size(egui::pos2(rect.max.x - w, rect.min.y), vec2(w, rect.height())), 0.0, color);
}

/// The framed parchment board every page is written on.
pub fn paint_board(painter: &egui::Painter, rect: Rect, ctx: &egui::Context) {
    let t = t();
    painter.rect_filled(rect, 0.0, t.line);
    painter.rect_filled(rect.shrink(2.0), 0.0, Color32::from_rgb(0xc9, 0x7d, 0x2e));
    ring(painter, rect.shrink(2.0), 2.0, Color32::from_rgb(0xea, 0xa6, 0x4e));
    painter.rect_filled(rect.shrink(8.0), 0.0, t.line);
    tile(painter, rect.shrink(10.0), &paper_texture(ctx), 4.0);
    ring(painter, rect.shrink(10.0), 2.0, Color32::from_black_alpha(36));
    for corner in [rect.left_top(), rect.right_top() - vec2(2.0, 0.0), rect.left_bottom() - vec2(0.0, 2.0), rect.right_bottom() - vec2(2.0, 2.0)] {
        painter.rect_filled(Rect::from_min_size(corner, vec2(2.0, 2.0)), 0.0, t.nav);
    }
}

/// The ledge the status line sits on.
pub fn paint_ledge(painter: &egui::Painter, rect: Rect) {
    let t = t();
    painter.rect_filled(rect, 0.0, t.status_bar);
    painter.rect_filled(Rect::from_min_size(rect.min, vec2(rect.width(), 4.0)), 0.0, t.line);
    painter.rect_filled(Rect::from_min_size(rect.min + vec2(0.0, 4.0), vec2(rect.width(), 2.0)), 0.0, Color32::from_white_alpha(70));
}

/// A pixel box: a dark outline with notched corners, a flat fill, a light and a shaded lip.
pub fn bevel_shapes(rect: Rect, fill: Color32, raised: bool, edge: f32) -> Vec<Shape> {
    let t = t();
    let (light, dark) = (Color32::from_white_alpha(120), Color32::from_black_alpha(50));
    let (a, b) = if raised { (light, dark) } else { (dark, light) };
    let inner = rect.shrink(2.0);
    let side = |r: Rect, c: Color32| Shape::rect_filled(r, 0.0, c);
    vec![
        side(rect.shrink2(vec2(2.0, 0.0)), t.line),
        side(rect.shrink2(vec2(0.0, 2.0)), t.line),
        side(inner, fill),
        side(Rect::from_min_size(inner.min, vec2(inner.width(), edge)), a),
        side(Rect::from_min_size(inner.min, vec2(edge, inner.height())), a),
        side(Rect::from_min_size(egui::pos2(inner.min.x, inner.max.y - edge), vec2(inner.width(), edge)), b),
        side(Rect::from_min_size(egui::pos2(inner.max.x - edge, inner.min.y), vec2(edge, inner.height())), b),
    ]
}

/// An inset on the parchment: the same box, pressed in.
fn panel_shapes(rect: Rect) -> Vec<Shape> {
    bevel_shapes(rect, t().card, false, 2.0)
}

/// Light on the top edge and a faint one below: what makes glass read as glass.
fn glass_edges(rect: Rect, radius: f32) -> Vec<Shape> {
    let r = radius.min(rect.height() / 2.0);
    vec![
        Shape::line_segment([egui::pos2(rect.min.x + r, rect.min.y + 1.0), egui::pos2(rect.max.x - r, rect.min.y + 1.0)], Stroke::new(1.0, Color32::from_white_alpha(46))),
        Shape::line_segment([egui::pos2(rect.min.x + r, rect.max.y - 0.5), egui::pos2(rect.max.x - r, rect.max.y - 0.5)], Stroke::new(1.0, Color32::from_black_alpha(60))),
    ]
}

pub fn card<R>(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    let t = t();
    let mut frame = egui::Frame::new()
        .fill(if t.bevel { Color32::TRANSPARENT } else { t.card })
        .corner_radius(CornerRadius::same(t.card_radius))
        .inner_margin(egui::Margin::same(16));
    if t.glass {
        frame = frame.stroke(Stroke::new(1.0, t.line)).shadow(egui::Shadow {
            offset: [0, 6],
            blur: 20,
            spread: 0,
            color: Color32::from_black_alpha(70),
        });
    }
    let slot = ui.painter().add(Shape::Noop);
    let mut prepared = frame.begin(ui);
    prepared.content_ui.set_width(prepared.content_ui.available_width());
    let out = add(&mut prepared.content_ui);
    let rect = prepared.frame.outer_rect(prepared.content_ui.min_rect());
    if t.bevel {
        ui.painter().set(slot, Shape::Vec(panel_shapes(rect)));
    } else if t.glass {
        ui.painter().set(slot, Shape::Vec(glass_edges(rect, f32::from(t.card_radius))));
    }
    prepared.end(ui);
    out
}

/// A well the text sits in: the log, the device code.
pub fn well<R>(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    let t = t();
    let fill = if t.bevel {
        Color32::from_rgb(0xff, 0xf8, 0xe0)
    } else {
        Color32::from_rgb(0x0a, 0x0c, 0x12)
    };
    egui::Frame::new()
        .fill(fill)
        .corner_radius(CornerRadius::same(t.control_radius.max(if t.bevel { 0 } else { 8 })))
        .inner_margin(egui::Margin::same(14))
        .stroke(Stroke::new(if t.bevel { 2.0 } else { 1.0 }, t.line))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            add(ui)
        })
        .inner
}

pub fn title(ui: &mut egui::Ui, text: &str, subtitle: &str) {
    let t = t();
    let size = if t.bevel { 28.0 } else { 24.0 };
    let galley = ui.painter().layout_no_wrap(text.to_owned(), face(size), t.text);
    let (rect, _) = ui.allocate_exact_size(galley.size(), Sense::hover());
    if t.bevel {
        ui.painter().galley(rect.min + vec2(2.0, 2.0), ui.painter().layout_no_wrap(text.to_owned(), face(size), Color32::from_rgb(0xd9, 0xa8, 0x62)), t.text);
    }
    ui.painter().galley(rect.min, galley, t.text);
    if !subtitle.is_empty() {
        ui.add_space(-6.0);
        ui.label(muted(subtitle));
        ui.add_space(4.0);
    }
}

pub fn section(ui: &mut egui::Ui, text: &str) {
    ui.add_space(4.0);
    ui.label(RichText::new(text).font(face(15.0)).strong());
}

pub fn muted(text: impl Into<String>) -> RichText {
    RichText::new(text.into()).color(t().muted).size(12.5)
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Primary,
    Secondary,
    Ghost,
    Danger,
}

pub fn mix_colors(a: Color32, b: Color32, f: f32) -> Color32 {
    mix(a, b, f)
}

fn mix(a: Color32, b: Color32, f: f32) -> Color32 {
    let l = |x: u8, y: u8| (f32::from(x) + (f32::from(y) - f32::from(x)) * f).round() as u8;
    Color32::from_rgba_unmultiplied(l(a.r(), b.r()), l(a.g(), b.g()), l(a.b(), b.b()), l(a.a(), b.a()))
}

/// Every button of the window: one shape language per look, a short ease between states.
pub fn action(ui: &mut egui::Ui, kind: Kind, icon: Option<Icon>, text: &str, enabled: bool, min: Vec2) -> egui::Response {
    let t = t();
    let big = kind == Kind::Primary;
    let size = if big { 17.0 } else { 14.0 } * if t.bevel { 1.1 } else { 1.0 };
    let color = match kind {
        Kind::Primary => t.on_accent,
        Kind::Secondary => t.text,
        Kind::Ghost => mix(t.muted, t.text, 0.45),
        Kind::Danger => t.danger,
    };
    let font = face(size);
    let galley = ui.painter().layout_no_wrap(text.to_owned(), font.clone(), color);
    let icon_w = if icon.is_some() { size + 8.0 } else { 0.0 };
    let pad = vec2(if big { 22.0 } else { 14.0 }, if big { 12.0 } else { 7.0 });
    let wanted = vec2(galley.size().x + icon_w + pad.x * 2.0, galley.size().y.max(size) + pad.y * 2.0);
    let size_final = vec2(wanted.x.max(min.x), wanted.y.max(min.y));
    let (rect, response) = ui.allocate_exact_size(size_final, if enabled { Sense::click() } else { Sense::hover() });
    let hovered = enabled && response.hovered();
    let pressed = enabled && response.is_pointer_button_down_on();
    let lift = ui.ctx().animate_value_with_time(response.id.with("hover"), if hovered { 1.0 } else { 0.0 }, 0.12);
    let (rest, over) = match kind {
        Kind::Primary => (t.accent, mix(t.accent, Color32::WHITE, 0.14)),
        Kind::Secondary => (t.control, t.control_hover),
        Kind::Ghost | Kind::Danger => (Color32::TRANSPARENT, t.control),
    };
    let mut fill = mix(rest, over, lift);
    if pressed {
        fill = mix(fill, Color32::BLACK, 0.18);
    }
    if !enabled {
        fill = mix(fill, t.bg, 0.55);
    }
    let painter = ui.painter();
    let radius = f32::from(t.control_radius);
    if t.bevel {
        let raised = !pressed;
        let base = if kind == Kind::Ghost || kind == Kind::Danger { mix(t.control, t.control_hover, lift) } else { fill };
        painter.add(Shape::Vec(bevel_shapes(rect, base, raised, t.border)));
    } else {
        painter.rect_filled(rect, radius, fill);
        if t.glass && kind != Kind::Ghost && kind != Kind::Danger {
            painter.rect_stroke(rect, radius, Stroke::new(1.0, Color32::from_white_alpha(if kind == Kind::Primary { 60 } else { 28 })), egui::StrokeKind::Inside);
        }
        if big && t.glass {
            for (grow, alpha) in [(1.0, 30u8), (2.5, 16), (4.0, 7)] {
                painter.rect_stroke(rect.expand(grow), radius + grow, Stroke::new(1.0, Color32::from_rgba_unmultiplied(t.accent.r(), t.accent.g(), t.accent.b(), alpha)), egui::StrokeKind::Outside);
            }
        }
    }
    if response.has_focus() {
        painter.rect_stroke(rect.expand(2.0), radius, Stroke::new(2.0, t.accent), egui::StrokeKind::Outside);
    }
    let text_color = if !enabled {
        mix(color, t.bg, 0.5)
    } else if kind == Kind::Ghost {
        mix(mix(t.muted, t.text, 0.45), t.text, lift)
    } else {
        color
    };
    let nudge = if t.bevel && pressed { 1.0 } else { 0.0 };
    let total = icon_w + galley.size().x;
    let mut x = rect.center().x - total / 2.0;
    let y = rect.center().y + nudge;
    if let Some(icon) = icon {
        let r = Rect::from_center_size(egui::pos2(x + size / 2.0, y), vec2(size, size));
        icons::paint(painter, icon, r, text_color);
        x += icon_w;
    }
    if t.bevel && big {
        painter.galley(egui::pos2(x + 1.0, y - galley.size().y / 2.0 + 1.0), painter.layout_no_wrap(text.to_owned(), font.clone(), Color32::from_rgb(0x1b, 0x3d, 0x0c)), text_color);
    }
    painter.galley(egui::pos2(x, y - galley.size().y / 2.0), painter.layout_no_wrap(text.to_owned(), font, text_color), text_color);
    response
}

/// Rows of choices in one track; returns the index the player picked this frame.
pub fn segmented(ui: &mut egui::Ui, options: &[String], selected: usize) -> Option<usize> {
    let t = t();
    let font = face(13.5);
    let widths: Vec<f32> = options
        .iter()
        .map(|o| ui.painter().layout_no_wrap(o.clone(), font.clone(), t.text).size().x + 30.0)
        .collect();
    let seg = widths.iter().copied().fold(78.0, f32::max);
    let size = vec2(seg * options.len() as f32 + 6.0, 36.0);
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    let radius = f32::from(t.control_radius) + 2.0;
    if t.bevel {
        ui.painter().add(Shape::Vec(bevel_shapes(rect, t.panel, false, t.border)));
    } else {
        ui.painter().rect_filled(rect, radius, if t.glass { Color32::from_black_alpha(70) } else { Color32::from_black_alpha(80) });
    }
    let mut picked = None;
    for (i, option) in options.iter().enumerate() {
        let cell = Rect::from_min_size(rect.min + vec2(3.0 + seg * i as f32, 3.0), vec2(seg, 30.0));
        let response = ui.interact(cell, ui.make_persistent_id(("segment", i, option)), Sense::click());
        let on = i == selected;
        let lift = ui.ctx().animate_value_with_time(response.id.with("seg"), if response.hovered() { 1.0 } else { 0.0 }, 0.12);
        let painter = ui.painter();
        if on {
            if t.bevel {
                painter.add(Shape::Vec(bevel_shapes(cell, t.accent, true, t.border)));
            } else {
                painter.rect_filled(cell, radius - 2.0, t.accent_dim);
                painter.rect_stroke(cell, radius - 2.0, Stroke::new(1.0, t.accent), egui::StrokeKind::Inside);
            }
        } else if lift > 0.01 {
            painter.rect_filled(cell, if t.bevel { 0.0 } else { radius - 2.0 }, if t.bevel { Color32::from_black_alpha((20.0 * lift) as u8) } else { Color32::from_white_alpha((14.0 * lift) as u8) });
        }
        let color = if on && t.bevel { t.on_accent } else if on { t.text } else { mix(t.muted, t.text, lift) };
        let galley = painter.layout_no_wrap(option.clone(), font.clone(), color);
        painter.galley(cell.center() - galley.size() / 2.0, galley, color);
        if response.clicked() && !on {
            picked = Some(i);
        }
    }
    picked
}

pub fn dot(ui: &mut egui::Ui, color: Color32, size: f32) {
    let (rect, _) = ui.allocate_exact_size(vec2(size, size), Sense::hover());
    if t().bevel {
        ui.painter().rect_filled(rect, 0.0, color);
        ui.painter().rect_stroke(rect, 0.0, Stroke::new(2.0, t().line), egui::StrokeKind::Outside);
    } else {
        ui.painter().circle_filled(rect.center(), size / 2.0, color);
    }
}

pub fn avatar(ui: &mut egui::Ui, name: &str, head: Option<&egui::TextureHandle>, size: f32) {
    let t = t();
    let (rect, _) = ui.allocate_exact_size(vec2(size, size), Sense::hover());
    if let Some(head) = head {
        let radius = if t.bevel { 0.0 } else { size * 0.16 };
        egui::Image::new(head).corner_radius(radius).paint_at(ui, rect);
        if t.bevel {
            ui.painter().rect_stroke(rect, 0.0, Stroke::new(2.0, t.line), egui::StrokeKind::Outside);
        }
        return;
    }
    let initial = name.chars().next().map_or('?', |c| c.to_ascii_uppercase());
    if t.bevel {
        ui.painter().rect_filled(rect, 0.0, t.accent_dim);
        ui.painter().rect_stroke(rect, 0.0, Stroke::new(2.0, t.line), egui::StrokeKind::Outside);
    } else {
        ui.painter().circle_filled(rect.center(), size / 2.0, t.accent_dim);
        ui.painter().circle_stroke(rect.center(), size / 2.0, Stroke::new(2.0, t.accent));
    }
    ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, initial, face(size * 0.5), t.text);
}

/// A quiet centred message with a drawn icon, for lists that have nothing in them yet.
pub fn empty_state(ui: &mut egui::Ui, icon: Icon, text: &str) {
    ui.vertical_centered(|ui| {
        ui.add_space(14.0);
        icons::icon(ui, icon, 28.0, t().muted);
        ui.add_space(4.0);
        ui.label(muted(text));
        ui.add_space(14.0);
    });
}
