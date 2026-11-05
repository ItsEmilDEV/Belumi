use eframe::egui::{Color32, Vec2, vec2};
use std::sync::atomic::{AtomicU8, Ordering};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Style {
    Modern,
    Glass,
    Minecraft,
}

impl Style {
    pub const ALL: [Style; 3] = [Style::Modern, Style::Glass, Style::Minecraft];

    pub fn parse(id: &str) -> Self {
        match id.trim().to_ascii_lowercase().as_str() {
            "glass" => Self::Glass,
            "minecraft" => Self::Minecraft,
            _ => Self::Modern,
        }
    }

    pub fn id(self) -> &'static str {
        match self {
            Self::Modern => "modern",
            Self::Glass => "glass",
            Self::Minecraft => "minecraft",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Modern => "Modern",
            Self::Glass => "Modern Glass",
            Self::Minecraft => "Minecraft",
        }
    }
}

static CURRENT: AtomicU8 = AtomicU8::new(0);

pub fn set(style: Style) {
    CURRENT.store(style as u8, Ordering::Relaxed);
}

pub fn current() -> Style {
    match CURRENT.load(Ordering::Relaxed) {
        1 => Style::Glass,
        2 => Style::Minecraft,
        _ => Style::Modern,
    }
}

/// The colours and shapes of one style (DESIGN.md).
#[derive(Clone, Copy, Debug)]
pub struct Theme {
    pub bg: Color32,
    pub panel: Color32,
    pub card: Color32,
    pub control: Color32,
    pub control_hover: Color32,
    pub line: Color32,
    pub accent: Color32,
    pub accent_dim: Color32,
    pub on_accent: Color32,
    pub text: Color32,
    pub muted: Color32,
    pub ok: Color32,
    pub warn: Color32,
    pub danger: Color32,
    pub card_radius: u8,
    pub control_radius: u8,
    pub border: f32,
    pub button_padding: Vec2,
    /// Square, bevelled and heavy-bordered (Minecraft).
    pub bevel: bool,
    /// The background is a gradient painted behind translucent panels.
    pub glass: bool,
}

fn white(alpha: u8) -> Color32 {
    Color32::from_white_alpha(alpha)
}

pub fn theme(style: Style) -> Theme {
    match style {
        Style::Modern => Theme {
            bg: Color32::from_rgb(0x0e, 0x10, 0x15),
            panel: Color32::from_rgb(0x14, 0x17, 0x1e),
            card: Color32::from_rgb(0x1b, 0x1f, 0x29),
            control: Color32::from_rgb(0x26, 0x2b, 0x38),
            control_hover: Color32::from_rgb(0x31, 0x38, 0x4a),
            line: Color32::from_rgb(0x24, 0x28, 0x33),
            accent: Color32::from_rgb(0x9b, 0x5c, 0xff),
            accent_dim: Color32::from_rgb(0x2a, 0x1b, 0x52),
            on_accent: Color32::WHITE,
            text: Color32::from_rgb(0xe6, 0xe8, 0xee),
            muted: Color32::from_rgb(0x9a, 0xa1, 0xb2),
            ok: Color32::from_rgb(0x4a, 0xde, 0x80),
            warn: Color32::from_rgb(0xfb, 0xbf, 0x24),
            danger: Color32::from_rgb(0xf8, 0x71, 0x71),
            card_radius: 12,
            control_radius: 8,
            border: 1.0,
            button_padding: vec2(14.0, 7.0),
            bevel: false,
            glass: false,
        },
        Style::Glass => Theme {
            bg: Color32::from_rgb(0x0b, 0x12, 0x20),
            panel: white(14),
            card: white(20),
            control: white(28),
            control_hover: white(42),
            line: white(36),
            accent: Color32::from_rgb(0x95, 0x66, 0xf6),
            accent_dim: Color32::from_rgba_unmultiplied(0xa9, 0x81, 0xff, 56),
            on_accent: Color32::WHITE,
            text: Color32::from_rgb(0xf0, 0xee, 0xfa),
            muted: Color32::from_rgb(0xb4, 0xb0, 0xcc),
            ok: Color32::from_rgb(0x4a, 0xde, 0x80),
            warn: Color32::from_rgb(0xfb, 0xbf, 0x24),
            danger: Color32::from_rgb(0xf8, 0x71, 0x71),
            card_radius: 16,
            control_radius: 10,
            border: 1.0,
            button_padding: vec2(16.0, 8.0),
            bevel: false,
            glass: true,
        },
        Style::Minecraft => Theme {
            bg: Color32::from_rgb(0x3b, 0x3b, 0x3b),
            panel: Color32::from_rgb(0x2b, 0x2b, 0x2b),
            card: Color32::from_rgb(0x4a, 0x4a, 0x4a),
            control: Color32::from_rgb(0x52, 0x52, 0x52),
            control_hover: Color32::from_rgb(0x60, 0x60, 0x60),
            line: Color32::from_rgb(0x10, 0x10, 0x10),
            accent: Color32::from_rgb(0x4a, 0x93, 0x22),
            accent_dim: Color32::from_rgb(0x2f, 0x5a, 0x17),
            on_accent: Color32::WHITE,
            text: Color32::from_rgb(0xf2, 0xf2, 0xf2),
            muted: Color32::from_rgb(0xdc, 0xdc, 0xdc),
            ok: Color32::from_rgb(0x7c, 0xd6, 0x4b),
            warn: Color32::from_rgb(0xff, 0xd1, 0x3a),
            danger: Color32::from_rgb(0xff, 0xc2, 0xb5),
            card_radius: 0,
            control_radius: 0,
            border: 2.0,
            button_padding: vec2(14.0, 8.0),
            bevel: true,
            glass: false,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn luminance(c: Color32) -> f32 {
        let f = |v: u8| {
            let v = f32::from(v) / 255.0;
            if v <= 0.03928 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) }
        };
        0.2126 * f(c.r()) + 0.7152 * f(c.g()) + 0.0722 * f(c.b())
    }

    fn over(top: Color32, bottom: Color32) -> Color32 {
        let a = f32::from(top.a()) / 255.0;
        let mix = |t: u8, b: u8| (f32::from(t) * a + f32::from(b) * (1.0 - a)) as u8;
        Color32::from_rgb(mix(top.r(), bottom.r()), mix(top.g(), bottom.g()), mix(top.b(), bottom.b()))
    }

    fn contrast(a: Color32, b: Color32) -> f32 {
        let (x, y) = (luminance(a), luminance(b));
        (x.max(y) + 0.05) / (x.min(y) + 0.05)
    }

    #[test]
    fn styles_round_trip_through_their_ids() {
        for s in Style::ALL {
            assert_eq!(Style::parse(s.id()), s);
        }
        assert_eq!(Style::parse("whatever"), Style::Modern);
    }

    #[test]
    fn text_is_readable_on_every_surface_of_every_style() {
        for style in Style::ALL {
            let t = theme(style);
            let ground = if t.glass { over(white(14), t.bg) } else { t.bg };
            for (name, surface) in [
                ("bg", ground),
                ("panel", over(t.panel, t.bg)),
                ("card", over(t.card, t.bg)),
                ("control", over(t.control, over(t.card, t.bg))),
                ("control hover", over(t.control_hover, over(t.card, t.bg))),
            ] {
                assert!(
                    contrast(t.text, surface) >= 4.5,
                    "{style:?} text on {name}: {}",
                    contrast(t.text, surface)
                );
                assert!(
                    contrast(t.muted, surface) >= 4.5,
                    "{style:?} muted on {name}: {}",
                    contrast(t.muted, surface)
                );
            }
            for (name, surface) in [("control", over(t.control, over(t.card, t.bg))), ("card", over(t.card, t.bg))] {
                assert!(
                    contrast(t.danger, surface) >= 4.5,
                    "{style:?} danger on {name}: {}",
                    contrast(t.danger, surface)
                );
            }
            assert!(
                contrast(t.on_accent, t.accent) >= 3.0,
                "{style:?} on accent: {}",
                contrast(t.on_accent, t.accent)
            );
        }
    }
}
