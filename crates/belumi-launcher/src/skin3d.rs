//! A small software renderer for the player's skin: boxes with the skin's own texels, a waving pose,
//! and a head that follows the cursor. No assets ship with it; a plain placeholder is painted when no skin is cached.

use eframe::egui::{self, Color32, Mesh, Pos2, Rect, Shape, TextureHandle, TextureOptions, pos2};

type V3 = [f32; 3];

pub struct Skin {
    base: TextureHandle,
    overlay: TextureHandle,
    slim: bool,
}

impl Skin {
    pub fn new(ctx: &egui::Context, key: &str, rgba: &[u8]) -> Self {
        let slim = rgba.len() >= 64 * 64 * 4 && rgba[(20 * 64 + 54) * 4 + 3] == 0 && rgba[(20 * 64 + 55) * 4 + 3] == 0;
        let mut opaque = rgba.to_vec();
        for px in opaque.chunks_mut(4) {
            px[3] = 255;
        }
        let load = |name: String, data: &[u8]| {
            ctx.load_texture(name, egui::ColorImage::from_rgba_unmultiplied([64, 64], data), TextureOptions::NEAREST)
        };
        Self { base: load(format!("{key}-base"), &opaque), overlay: load(format!("{key}-overlay"), rgba), slim }
    }
}

/// A flat stand-in skin in the accent colour, so the stage is never empty (offline, first start).
pub fn placeholder(accent: Color32) -> Vec<u8> {
    let mut img = vec![0u8; 64 * 64 * 4];
    let mut fill = |u: usize, v: usize, w: usize, h: usize, d: usize, c: Color32| {
        for (x0, y0, fw, fh) in [
            (u + d, v, w * 2, d),
            (u, v + d, d * 2 + w * 2, h),
        ] {
            for y in y0..y0 + fh {
                for x in x0..x0 + fw {
                    img[(y * 64 + x) * 4..][..4].copy_from_slice(&c.to_array());
                }
            }
        }
    };
    let skin = Color32::from_rgb(0xd9, 0xa7, 0x7e);
    let hair = Color32::from_rgb(0x4a, 0x2f, 0x1c);
    let pants = Color32::from_rgb(0x2a, 0x2f, 0x52);
    fill(0, 0, 8, 8, 8, skin);
    fill(16, 16, 8, 12, 4, accent);
    fill(40, 16, 4, 12, 4, accent);
    fill(32, 48, 4, 12, 4, accent);
    fill(0, 16, 4, 12, 4, pants);
    fill(16, 48, 4, 12, 4, pants);
    let mut put = |x: usize, y: usize, c: Color32| img[(y * 64 + x) * 4..][..4].copy_from_slice(&c.to_array());
    for y in 0..8 {
        for x in 8..24 {
            put(x, y, hair);
        }
    }
    for y in 8..16 {
        for x in (0..8).chain(16..32) {
            put(x, y, hair);
        }
        for x in 8..16 {
            put(x, y, if y < 10 { hair } else { skin });
        }
    }
    let eye = Color32::from_rgb(0x2a, 0x2a, 0x3a);
    for (x, y) in [(10, 12), (13, 12)] {
        put(x, y, eye);
    }
    img
}

fn add(a: V3, b: V3) -> V3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
fn sub(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn cross(a: V3, b: V3) -> V3 {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}
fn rot_x(v: V3, a: f32) -> V3 {
    let (s, c) = a.sin_cos();
    [v[0], v[1] * c - v[2] * s, v[1] * s + v[2] * c]
}
fn rot_y(v: V3, a: f32) -> V3 {
    let (s, c) = a.sin_cos();
    [v[0] * c + v[2] * s, v[1], -v[0] * s + v[2] * c]
}
fn rot_z(v: V3, a: f32) -> V3 {
    let (s, c) = a.sin_cos();
    [v[0] * c - v[1] * s, v[0] * s + v[1] * c, v[2]]
}

struct Part {
    size: V3,
    uv: (f32, f32),
    pivot: V3,
    centre: V3,
    rot: V3,
}

struct Face {
    corners: [V3; 4],
    uvs: [Pos2; 4],
    normal: V3,
    depth: f32,
    overlay: bool,
}

fn faces(part: &Part, inflate: f32, overlay: bool, out: &mut Vec<Face>, turn: &dyn Fn(V3) -> V3) {
    let [w, h, d] = part.size;
    let (hx, hy, hz) = (w / 2.0 + inflate, h / 2.0 + inflate, d / 2.0 + inflate);
    let (u, v) = part.uv;
    let uvp = |x: f32, y: f32| pos2(x / 64.0, y / 64.0);
    // origin corner, u axis, v axis, and the texel rectangle (x, y, width, height)
    let defs: [(V3, V3, V3, (f32, f32, f32, f32)); 6] = [
        ([-hx, hy, hz], [2.0 * hx, 0.0, 0.0], [0.0, -2.0 * hy, 0.0], (u + d, v + d, w, h)),
        ([hx, hy, -hz], [-2.0 * hx, 0.0, 0.0], [0.0, -2.0 * hy, 0.0], (u + 2.0 * d + w, v + d, w, h)),
        ([-hx, hy, -hz], [0.0, 0.0, 2.0 * hz], [0.0, -2.0 * hy, 0.0], (u, v + d, d, h)),
        ([hx, hy, hz], [0.0, 0.0, -2.0 * hz], [0.0, -2.0 * hy, 0.0], (u + d + w, v + d, d, h)),
        ([-hx, hy, -hz], [2.0 * hx, 0.0, 0.0], [0.0, 0.0, 2.0 * hz], (u + d, v, w, d)),
        ([-hx, -hy, hz], [2.0 * hx, 0.0, 0.0], [0.0, 0.0, -2.0 * hz], (u + d + w, v, w, d)),
    ];
    for (p0, uax, vax, (tx, ty, tw, th)) in defs {
        let local = [p0, add(p0, uax), add(add(p0, uax), vax), add(p0, vax)];
        let mut corners = [[0.0; 3]; 4];
        for (i, c) in local.iter().enumerate() {
            let placed = add(part.centre, *c);
            let r = rot_z(placed, part.rot[2]);
            let r = rot_x(r, part.rot[0]);
            corners[i] = turn(add(r, part.pivot));
        }
        let normal = cross(sub(corners[3], corners[0]), sub(corners[1], corners[0]));
        let depth = corners.iter().map(|c| c[2]).sum::<f32>() / 4.0 + if overlay { 0.4 } else { 0.0 };
        out.push(Face {
            corners,
            uvs: [uvp(tx, ty), uvp(tx + tw, ty), uvp(tx + tw, ty + th), uvp(tx, ty + th)],
            normal,
            depth,
            overlay,
        });
    }
}

/// Draws the character into `area`, feet at the bottom, and keeps asking for frames while it moves.
pub fn paint(ui: &egui::Ui, area: Rect, skin: &Skin) {
    let time = ui.input(|i| i.time) as f32;
    let cursor = ui.input(|i| i.pointer.hover_pos());
    let yaw = -0.5 + 0.16 * (time * 0.9).sin();
    let pitch = 0.1;
    let turn = move |v: V3| rot_x(rot_y(v, yaw), pitch);

    let arm_w = if skin.slim { 3.0 } else { 4.0 };
    let arm_x = 4.0 + arm_w / 2.0;
    let centre = area.center();
    let (look_x, look_y) = cursor.map_or((0.0, 0.0), |p| (((p.x - centre.x) / 260.0).clamp(-1.0, 1.0), ((p.y - area.min.y) / 320.0).clamp(-1.0, 1.0)));
    let head_yaw = look_x * 0.7 - yaw * 0.6;
    let bob = 0.35 * (time * 1.8).sin();
    let wave = -(2.55 + 0.22 * (time * 5.0).sin());

    let head = |p: V3| rot_y(rot_x(p, 0.0), head_yaw);
    let mut all = Vec::new();
    let parts: [(Part, Part, f32, bool); 6] = [
        (
            Part { size: [8.0, 8.0, 8.0], uv: (0.0, 0.0), pivot: [0.0, 24.0 + bob, 0.0], centre: [0.0, 4.0, 0.0], rot: [look_y * 0.35, 0.0, 0.0] },
            Part { size: [8.0, 8.0, 8.0], uv: (32.0, 0.0), pivot: [0.0, 24.0 + bob, 0.0], centre: [0.0, 4.0, 0.0], rot: [look_y * 0.35, 0.0, 0.0] },
            0.5,
            true,
        ),
        (
            Part { size: [8.0, 12.0, 4.0], uv: (16.0, 16.0), pivot: [0.0, 12.0 + bob, 0.0], centre: [0.0, 6.0, 0.0], rot: [0.0; 3] },
            Part { size: [8.0, 12.0, 4.0], uv: (16.0, 32.0), pivot: [0.0, 12.0 + bob, 0.0], centre: [0.0, 6.0, 0.0], rot: [0.0; 3] },
            0.25,
            false,
        ),
        (
            Part { size: [arm_w, 12.0, 4.0], uv: (40.0, 16.0), pivot: [-arm_x, 22.0 + bob, 0.0], centre: [0.0, -4.0, 0.0], rot: [0.0, 0.0, wave] },
            Part { size: [arm_w, 12.0, 4.0], uv: (40.0, 32.0), pivot: [-arm_x, 22.0 + bob, 0.0], centre: [0.0, -4.0, 0.0], rot: [0.0, 0.0, wave] },
            0.25,
            false,
        ),
        (
            Part { size: [arm_w, 12.0, 4.0], uv: (32.0, 48.0), pivot: [arm_x, 22.0 + bob, 0.0], centre: [0.0, -4.0, 0.0], rot: [0.1, 0.0, 0.14] },
            Part { size: [arm_w, 12.0, 4.0], uv: (48.0, 48.0), pivot: [arm_x, 22.0 + bob, 0.0], centre: [0.0, -4.0, 0.0], rot: [0.1, 0.0, 0.14] },
            0.25,
            false,
        ),
        (
            Part { size: [4.0, 12.0, 4.0], uv: (0.0, 16.0), pivot: [-2.0, 12.0 + bob, 0.0], centre: [0.0, -6.0, 0.0], rot: [-0.12, 0.0, -0.05] },
            Part { size: [4.0, 12.0, 4.0], uv: (0.0, 32.0), pivot: [-2.0, 12.0 + bob, 0.0], centre: [0.0, -6.0, 0.0], rot: [-0.12, 0.0, -0.05] },
            0.25,
            false,
        ),
        (
            Part { size: [4.0, 12.0, 4.0], uv: (16.0, 48.0), pivot: [2.0, 12.0 + bob, 0.0], centre: [0.0, -6.0, 0.0], rot: [0.16, 0.0, 0.05] },
            Part { size: [4.0, 12.0, 4.0], uv: (0.0, 48.0), pivot: [2.0, 12.0 + bob, 0.0], centre: [0.0, -6.0, 0.0], rot: [0.16, 0.0, 0.05] },
            0.25,
            false,
        ),
    ];
    for (base, over, inflate, is_head) in &parts {
        if *is_head {
            let around = |v: V3| turn(add(head(sub(v, [0.0, 24.0 + bob, 0.0])), [0.0, 24.0 + bob, 0.0]));
            faces(base, 0.0, false, &mut all, &around);
            faces(over, *inflate, true, &mut all, &around);
        } else {
            faces(base, 0.0, false, &mut all, &turn);
            faces(over, *inflate, true, &mut all, &turn);
        }
    }
    all.sort_by(|a, b| a.depth.total_cmp(&b.depth));

    let scale = area.height() / 36.0;
    let origin = pos2(area.center().x, area.max.y - 2.0 * scale);
    let light = [-0.3f32, 0.6, 0.75];
    let light_len = (light[0] * light[0] + light[1] * light[1] + light[2] * light[2]).sqrt();
    let painter = ui.painter_at(area.expand2(egui::vec2(0.0, 40.0)));
    for face in &all {
        if face.normal[2] <= 0.0 {
            continue;
        }
        let len = (face.normal[0].powi(2) + face.normal[1].powi(2) + face.normal[2].powi(2)).sqrt().max(1e-6);
        let lit = (face.normal[0] * light[0] + face.normal[1] * light[1] + face.normal[2] * light[2]) / (len * light_len);
        let shade = (0.6 + 0.4 * lit.max(0.0)).clamp(0.0, 1.0);
        let tint = Color32::from_gray((shade * 255.0) as u8);
        let texture = if face.overlay { &skin.overlay } else { &skin.base };
        let mut mesh = Mesh::with_texture(texture.id());
        for (c, uv) in face.corners.iter().zip(face.uvs) {
            mesh.vertices.push(egui::epaint::Vertex { pos: pos2(origin.x + c[0] * scale, origin.y - c[1] * scale), uv, color: tint });
        }
        mesh.indices.extend([0, 1, 2, 0, 2, 3]);
        painter.add(Shape::mesh(mesh));
    }
    ui.ctx().request_repaint_after(std::time::Duration::from_millis(33));
}
