use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

use eframe::egui::{FontData, FontDefinitions, FontFamily};

static WELCOME_FAMILY: OnceLock<FontFamily> = OnceLock::new();

fn font_candidates() -> Vec<PathBuf> {
    let v = vec![
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/fonts/YuGothM.ttc"),
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/fonts/YuGothic-Medium.ttf"),
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/fonts/YuGothicMedium.otf"),
        PathBuf::from("assets/fonts/YuGothM.ttc"),
        PathBuf::from("assets/fonts/YuGothic-Medium.ttf"),
    ];
    #[cfg(windows)]
    v.push(PathBuf::from(r"C:\Windows\Fonts\YuGothM.ttc"));
    v
}

/// call once from [`crate::gui::setup_style`] before [`eframe::egui::Context::set_fonts`]
pub fn setup(fonts: &mut FontDefinitions) {
    if WELCOME_FAMILY.get().is_some() {
        return;
    }
    let fam = if let Some(bytes) = font_candidates()
        .into_iter()
        .find_map(|p| std::fs::read(&p).ok())
    {
        fonts.font_data.insert(
            "yu_gothic_medium_welcome".to_owned(),
            Arc::new(FontData::from_owned(bytes)),
        );
        fonts.families.insert(
            FontFamily::Name("welcome_yu_gothic".into()),
            vec!["yu_gothic_medium_welcome".to_owned()],
        );
        FontFamily::Name("welcome_yu_gothic".into())
    } else {
        FontFamily::Name("lain_title".into())
    };
    let _ = WELCOME_FAMILY.set(fam);
}

pub fn family() -> FontFamily {
    WELCOME_FAMILY
        .get()
        .expect("welcome_font::setup must run in setup_style")
        .clone()
}
