//! entry

mod avr;
mod boot_video;
mod cycle_helper;
mod docs;
mod ed060sc4_display;
mod ed060sc4_sim;
mod editor;
mod gui;
mod sim_panel;
mod syntax;
mod toolbar;
mod upload_panel;
mod welcome;
mod welcome_font;
mod word_helper;

fn main() -> eframe::Result {
    let native_options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default().with_fullscreen(true),
        ..Default::default()
    };
    eframe::run_native(
        "Lain Studio",
        native_options,
        Box::new(|cc| Ok(Box::new(gui::LainApp::new(cc)))),
    )
}
