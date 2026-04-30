//! entry

mod avr;
mod clipped_color_picker;
mod customization;
mod cost_helper;
mod cycle_helper;
mod docs;
mod editor;
mod gui;
mod modal_chrome;
mod peripherals;
mod real_uart;
mod sim_panel;
mod syntax;
mod theme;
mod toolbar;
mod uart_panel;
mod upload_panel;
mod waveforms;
mod word_helper;
mod wallpaper_filter;

fn main() -> eframe::Result {
    let native_options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default().with_fullscreen(true),
        ..Default::default()
    };
    eframe::run_native(
        "Full Metal Studio",
        native_options,
        Box::new(|cc| Ok(Box::new(gui::FullMetalApp::new(cc)))),
    )
}
