#![allow(dead_code)]

use eframe::egui::{self, Color32, Pos2, Rect, Shape, Stroke, Vec2};
use std::f32::consts::TAU;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Icon {
    Play,
    User,
    Users,
    Log,
    Settings,
    Refresh,
    Plus,
    Close,
    Trash,
    Server,
    World,
    Check,
    Warning,
    Folder,
    Shield,
    Download,
    Arrow,
    Cube,
}

pub const ALL: [Icon; 18] = [
    Icon::Play,
    Icon::User,
    Icon::Users,
    Icon::Log,
    Icon::Settings,
    Icon::Refresh,
    Icon::Plus,
    Icon::Close,
    Icon::Trash,
    Icon::Server,
    Icon::World,
    Icon::Check,
    Icon::Warning,
    Icon::Folder,
    Icon::Shield,
    Icon::Download,
    Icon::Arrow,
    Icon::Cube,
];

const GRID: f32 = 24.0;
const STROKE_AT_GRID: f32 = 1.75;
const GEAR_TEETH: usize = 8;

struct Pen {
    origin: Pos2,
    k: f32,
    stroke: Stroke,
    color: Color32,
    shapes: Vec<Shape>,
}

impl Pen {
    fn new(rect: Rect, color: Color32) -> Self {
        let size = rect.width().min(rect.height());
        let k = size / GRID;
        let origin = rect.center() - Vec2::splat(size / 2.0);
        Self {
            origin,
            k,
            stroke: Stroke::new(STROKE_AT_GRID * k, color),
            color,
            shapes: Vec::new(),
        }
    }

    fn pt(&self, x: f32, y: f32) -> Pos2 {
        self.origin + Vec2::new(x, y) * self.k
    }

    fn cap(&mut self, p: Pos2) {
        self.shapes
            .push(Shape::circle_filled(p, self.stroke.width / 2.0, self.color));
    }

    fn path(&mut self, pts: &[Pos2], closed: bool) {
        if pts.len() < 2 {
            return;
        }
        if closed {
            self.shapes
                .push(Shape::closed_line(pts.to_vec(), self.stroke));
            for &p in pts {
                self.cap(p);
            }
        } else {
            self.shapes.push(Shape::line(pts.to_vec(), self.stroke));
            self.cap(pts[0]);
            self.cap(pts[pts.len() - 1]);
        }
    }

    fn poly(&mut self, pts: &[(f32, f32)], closed: bool) {
        let v: Vec<Pos2> = pts.iter().map(|&(x, y)| self.pt(x, y)).collect();
        self.path(&v, closed);
    }

    fn line(&mut self, a: (f32, f32), b: (f32, f32)) {
        self.poly(&[a, b], false);
    }

    fn circle(&mut self, c: (f32, f32), r: f32) {
        self.shapes
            .push(Shape::circle_stroke(self.pt(c.0, c.1), r * self.k, self.stroke));
    }

    fn dot(&mut self, c: (f32, f32)) {
        let p = self.pt(c.0, c.1);
        self.shapes
            .push(Shape::circle_filled(p, self.stroke.width * 0.8, self.color));
    }

    fn rrect(&mut self, x: f32, y: f32, w: f32, h: f32, r: f32) {
        let rect = Rect::from_min_size(self.pt(x, y), Vec2::new(w, h) * self.k);
        self.shapes.push(Shape::rect_stroke(
            rect,
            r * self.k,
            self.stroke,
            egui::StrokeKind::Middle,
        ));
    }

    fn arc(&mut self, c: (f32, f32), r: f32, from_deg: f32, to_deg: f32) {
        let pts = arc_points(c, r, from_deg, to_deg, 20);
        self.poly(&pts, false);
    }
}

fn arc_points(c: (f32, f32), r: f32, from_deg: f32, to_deg: f32, steps: usize) -> Vec<(f32, f32)> {
    (0..=steps)
        .map(|i| {
            let t = i as f32 / steps as f32;
            let a = (from_deg + (to_deg - from_deg) * t).to_radians();
            (c.0 + r * a.cos(), c.1 + r * a.sin())
        })
        .collect()
}

fn gear_polygon(c: (f32, f32), outer: f32, inner: f32, teeth: usize) -> Vec<(f32, f32)> {
    let step = TAU / teeth as f32;
    let mut pts = Vec::with_capacity(teeth * 4);
    for i in 0..teeth {
        let mid = i as f32 * step;
        for (off, r) in [(-0.48, inner), (-0.3, outer), (0.3, outer), (0.48, inner)] {
            let a = mid + off * step;
            pts.push((c.0 + r * a.cos(), c.1 + r * a.sin()));
        }
    }
    pts
}

fn play_triangle() -> [(f32, f32); 3] {
    [(7.0, 4.5), (19.5, 12.0), (7.0, 19.5)]
}

pub fn shapes(icon: Icon, rect: Rect, color: Color32) -> Vec<Shape> {
    let mut p = Pen::new(rect, color);
    match icon {
        Icon::Play => {
            let tri: Vec<Pos2> = play_triangle().iter().map(|&(x, y)| p.pt(x, y)).collect();
            p.shapes
                .push(Shape::convex_polygon(tri.clone(), color, Stroke::NONE));
            p.path(&tri, true);
        }
        Icon::User => {
            p.circle((12.0, 8.0), 4.0);
            p.arc((12.0, 21.0), 8.0, 180.0, 360.0);
        }
        Icon::Users => {
            p.circle((9.0, 8.0), 3.5);
            p.arc((9.0, 20.0), 6.5, 180.0, 360.0);
            p.arc((16.0, 8.0), 3.5, -70.0, 70.0);
            p.arc((16.0, 20.0), 5.0, 270.0, 360.0);
        }
        Icon::Log => {
            p.rrect(5.0, 3.0, 14.0, 18.0, 2.5);
            p.line((8.5, 8.0), (15.5, 8.0));
            p.line((8.5, 12.0), (15.5, 12.0));
            p.line((8.5, 16.0), (12.5, 16.0));
        }
        Icon::Settings => {
            let g = gear_polygon((12.0, 12.0), 10.0, 7.8, GEAR_TEETH);
            p.poly(&g, true);
            p.circle((12.0, 12.0), 3.0);
        }
        Icon::Refresh => {
            p.arc((12.0, 12.0), 8.0, -20.0, -300.0);
            p.poly(&[(19.5, 4.0), (19.5, 9.26), (14.2, 9.26)], false);
        }
        Icon::Plus => {
            p.line((12.0, 5.0), (12.0, 19.0));
            p.line((5.0, 12.0), (19.0, 12.0));
        }
        Icon::Close => {
            p.line((6.0, 6.0), (18.0, 18.0));
            p.line((18.0, 6.0), (6.0, 18.0));
        }
        Icon::Trash => {
            p.line((4.0, 6.5), (20.0, 6.5));
            p.poly(&[(9.0, 6.5), (9.0, 4.0), (15.0, 4.0), (15.0, 6.5)], false);
            p.poly(&[(6.5, 6.5), (7.5, 20.0), (16.5, 20.0), (17.5, 6.5)], false);
            p.line((10.0, 10.5), (10.0, 16.0));
            p.line((14.0, 10.5), (14.0, 16.0));
        }
        Icon::Server => {
            p.rrect(3.5, 4.0, 17.0, 6.5, 2.0);
            p.rrect(3.5, 13.5, 17.0, 6.5, 2.0);
            p.dot((7.5, 7.25));
            p.dot((7.5, 16.75));
        }
        Icon::World => {
            p.circle((12.0, 12.0), 9.0);
            p.line((3.0, 12.0), (21.0, 12.0));
            let pts: Vec<(f32, f32)> = (0..24)
                .map(|i| {
                    let a = TAU * i as f32 / 24.0;
                    (12.0 + 4.0 * a.cos(), 12.0 + 9.0 * a.sin())
                })
                .collect();
            p.poly(&pts, true);
        }
        Icon::Check => {
            p.poly(&[(5.0, 12.5), (10.0, 17.5), (19.0, 7.0)], false);
        }
        Icon::Warning => {
            p.poly(&[(12.0, 3.8), (21.0, 19.5), (3.0, 19.5)], true);
            p.line((12.0, 9.5), (12.0, 14.0));
            p.dot((12.0, 17.0));
        }
        Icon::Folder => {
            p.poly(
                &[
                    (3.5, 5.5),
                    (9.0, 5.5),
                    (11.0, 8.0),
                    (20.5, 8.0),
                    (20.5, 19.0),
                    (3.5, 19.0),
                ],
                true,
            );
        }
        Icon::Shield => {
            p.poly(
                &[
                    (12.0, 3.0),
                    (19.5, 6.0),
                    (19.5, 11.5),
                    (16.5, 17.0),
                    (12.0, 21.0),
                    (7.5, 17.0),
                    (4.5, 11.5),
                    (4.5, 6.0),
                ],
                true,
            );
        }
        Icon::Download => {
            p.line((12.0, 4.0), (12.0, 15.0));
            p.poly(&[(7.0, 10.5), (12.0, 15.5), (17.0, 10.5)], false);
            p.poly(&[(4.0, 15.0), (4.0, 19.5), (20.0, 19.5), (20.0, 15.0)], false);
        }
        Icon::Arrow => {
            p.line((4.0, 12.0), (20.0, 12.0));
            p.poly(&[(13.5, 5.5), (20.0, 12.0), (13.5, 18.5)], false);
        }
        Icon::Cube => {
            p.poly(
                &[
                    (12.0, 2.5),
                    (20.0, 7.25),
                    (20.0, 16.75),
                    (12.0, 21.5),
                    (4.0, 16.75),
                    (4.0, 7.25),
                ],
                true,
            );
            p.poly(&[(4.0, 7.25), (12.0, 12.0), (20.0, 7.25)], false);
            p.line((12.0, 12.0), (12.0, 21.5));
        }
    }
    p.shapes
}

pub fn paint(painter: &egui::Painter, icon: Icon, rect: Rect, color: Color32) {
    painter.extend(shapes(icon, rect, color));
}

pub fn icon(ui: &mut egui::Ui, icon: Icon, size: f32, color: Color32) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(size), egui::Sense::hover());
    if ui.is_rect_visible(rect) {
        paint(ui.painter(), icon, rect, color);
    }
    response
}

pub fn gallery(ui: &mut egui::Ui, color: Color32) {
    egui::Grid::new("icon_gallery")
        .num_columns(6)
        .spacing([20.0, 16.0])
        .show(ui, |ui| {
            for (i, ic) in ALL.iter().enumerate() {
                ui.vertical_centered(|ui| {
                    ui.horizontal(|ui| {
                        for size in [16.0, 20.0, 24.0] {
                            icon(ui, *ic, size, color);
                        }
                    });
                    ui.label(format!("{ic:?}"));
                });
                if (i + 1) % 6 == 0 {
                    ui.end_row();
                }
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(size: f32) -> Rect {
        Rect::from_min_size(Pos2::new(10.0, 10.0), Vec2::splat(size))
    }

    #[test]
    fn every_icon_yields_shapes() {
        for size in [16.0, 20.0, 24.0] {
            for ic in ALL {
                assert!(!shapes(ic, rect(size), Color32::WHITE).is_empty(), "{ic:?}");
            }
        }
    }

    #[test]
    fn every_icon_paints_into_painter() {
        let ctx = egui::Context::default();
        let painter = egui::Painter::new(
            ctx.clone(),
            egui::LayerId::background(),
            Rect::from_min_size(Pos2::ZERO, Vec2::splat(100.0)),
        );
        for ic in ALL {
            paint(&painter, ic, rect(24.0), Color32::WHITE);
        }
    }

    #[test]
    fn all_lists_every_variant_once() {
        for (i, a) in ALL.iter().enumerate() {
            for b in &ALL[i + 1..] {
                assert_ne!(a, b);
            }
        }
    }

    #[test]
    fn gear_has_four_points_per_tooth_inside_bounds() {
        let g = gear_polygon((12.0, 12.0), 10.0, 7.8, GEAR_TEETH);
        assert_eq!(g.len(), GEAR_TEETH * 4);
        for (x, y) in g {
            assert!((2.0..=22.0).contains(&x) && (2.0..=22.0).contains(&y));
        }
    }

    #[test]
    fn play_triangle_inside_grid() {
        for (x, y) in play_triangle() {
            assert!((1.0..=23.0).contains(&x) && (1.0..=23.0).contains(&y));
        }
    }

    #[test]
    fn shapes_stay_inside_rect() {
        let r = rect(24.0);
        for ic in ALL {
            for s in shapes(ic, r, Color32::WHITE) {
                assert!(
                    r.expand(0.5).contains_rect(s.visual_bounding_rect()),
                    "{ic:?}"
                );
            }
        }
    }

    #[test]
    fn arc_endpoints() {
        let a = arc_points((0.0, 0.0), 2.0, 0.0, 90.0, 10);
        assert!((a[0].0 - 2.0).abs() < 1e-5);
        assert!((a[10].1 - 2.0).abs() < 1e-5);
    }
}
