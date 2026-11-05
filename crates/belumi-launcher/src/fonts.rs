#![allow(dead_code)]

use eframe::egui::{self, FontData, FontDefinitions, FontFamily, FontId};
use std::sync::Arc;

static INTER: &[u8] = include_bytes!("../assets/fonts/Inter.ttf");
static PIXELIFY: &[u8] = include_bytes!("../assets/fonts/PixelifySans.ttf");

fn family_ui() -> FontFamily {
    FontFamily::Name("ui".into())
}

fn family_pixel() -> FontFamily {
    FontFamily::Name("pixel".into())
}

fn definitions() -> FontDefinitions {
    let mut defs = FontDefinitions::default();
    defs.font_data
        .insert("Inter".into(), Arc::new(FontData::from_static(INTER)));
    defs.font_data.insert(
        "PixelifySans".into(),
        Arc::new(FontData::from_static(PIXELIFY)),
    );

    let fallback = defs
        .families
        .get(&FontFamily::Proportional)
        .cloned()
        .unwrap_or_default();

    let mut ui = vec!["Inter".to_owned()];
    ui.extend(fallback.iter().cloned());
    let mut pixel = vec!["PixelifySans".to_owned()];
    pixel.extend(fallback);

    defs.families.insert(family_ui(), ui);
    defs.families.insert(family_pixel(), pixel);
    defs
}

pub fn install(ctx: &egui::Context) {
    ctx.set_fonts(definitions());
}

pub fn ui_font(size: f32) -> FontId {
    FontId::new(size, family_ui())
}

pub fn pixel_font(size: f32) -> FontId {
    FontId::new(size, family_pixel())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn has_font_header(bytes: &[u8]) -> bool {
        matches!(
            bytes.get(..4),
            Some([0x00, 0x01, 0x00, 0x00]) | Some(b"OTTO") | Some(b"true")
        )
    }

    #[test]
    fn families_exist_and_are_populated() {
        let defs = definitions();
        for fam in [family_ui(), family_pixel()] {
            let list = defs.families.get(&fam).expect("family missing");
            assert!(!list.is_empty());
            for name in list {
                assert!(defs.font_data.contains_key(name));
            }
        }
        assert!(defs.families[&family_ui()].len() > 1);
    }

    #[test]
    fn font_bytes_are_valid() {
        for bytes in [INTER, PIXELIFY] {
            assert!(!bytes.is_empty());
            assert!(has_font_header(bytes));
        }
    }

    #[test]
    fn font_ids_use_named_families() {
        assert_eq!(ui_font(14.0).family, family_ui());
        assert_eq!(pixel_font(20.0).family, family_pixel());
    }
}
