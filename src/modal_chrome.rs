//! styling for file dialogs and confirmation modals (matches simulator / peripheral panels)
//! (mostly, not fully implimented)

use eframe::egui::{
    self, Button, Color32, CornerRadius, FontId, Frame, Margin, RichText, Stroke, TextEdit, Ui,
};

use crate::theme;

pub fn modal_window_frame() -> Frame {
    Frame::NONE
        .fill(theme::sim_surface())
        .stroke(Stroke::new(0.75, theme::sim_border()))
        .inner_margin(Margin::same(14))
        .corner_radius(CornerRadius::same(5))
}

pub fn modal_title(ui: &mut Ui, text: &str) {
    ui.label(
        RichText::new(text)
            .monospace()
            .size(13.0)
            .color(theme::start_green()),
    );
}

pub fn modal_body(ui: &mut Ui, text: &str) {
    ui.label(
        RichText::new(text)
            .monospace()
            .size(12.0)
            .color(theme::accent_dim()),
    );
}

pub fn modal_caption(ui: &mut Ui, text: &str) {
    ui.label(
        RichText::new(text)
            .monospace()
            .size(11.0)
            .color(theme::accent_dim()),
    );
}

pub fn modal_error(ui: &mut Ui, text: &str) {
    ui.label(
        RichText::new(text)
            .monospace()
            .size(11.5)
            .color(theme::err_red()),
    );
}

pub fn modal_single_line_edit(ui: &mut Ui, text: &mut String) {
    modal_single_line_edit_with_id(ui, text, None, f32::INFINITY);
}

pub fn modal_single_line_edit_with_id(
    ui: &mut Ui,
    text: &mut String,
    id: Option<egui::Id>,
    desired_width: f32,
) -> egui::Response {
    let inner = Frame::NONE
        .fill(theme::sim_surface_lift())
        .stroke(Stroke::new(0.75, theme::sim_border()))
        .inner_margin(Margin::symmetric(8, 6))
        .corner_radius(CornerRadius::same(4))
        .show(ui, |ui| {
            let mut te = TextEdit::singleline(text)
                .font(FontId::monospace(12.0))
                .desired_width(desired_width);
            if let Some(i) = id {
                te = te.id(i);
            }
            ui.add(te)
        });
    inner.inner
}

pub fn search_bar_frame() -> Frame {
    Frame::NONE
        .fill(theme::search_bg())
        .stroke(Stroke::new(0.75, theme::sim_border()))
        .inner_margin(Margin::symmetric(10, 8))
        .corner_radius(CornerRadius::same(5))
}

pub fn modal_btn_primary(ui: &mut Ui, label: &str) -> egui::Response {
    ui.add(
        Button::new(
            RichText::new(label)
                .monospace()
                .size(12.0)
                .color(Color32::BLACK),
        )
        .fill(theme::start_green_dim())
        .stroke(Stroke::new(1.0, theme::start_green()))
        .corner_radius(CornerRadius::same(5)),
    )
}

pub fn modal_btn_secondary(ui: &mut Ui, label: &str) -> egui::Response {
    ui.add(
        Button::new(
            RichText::new(label)
                .monospace()
                .size(12.0)
                .color(theme::start_green_dim()),
        )
        .fill(theme::sim_surface_lift())
        .stroke(Stroke::new(0.75, theme::sim_border()))
        .corner_radius(CornerRadius::same(5)),
    )
}

pub fn modal_btn_danger(ui: &mut Ui, label: &str) -> egui::Response {
    ui.add(
        Button::new(
            RichText::new(label)
                .monospace()
                .size(12.0)
                .color(theme::accent_dim()),
        )
        .fill(theme::sim_tab_active())
        .stroke(Stroke::new(0.75, theme::sim_border_bright()))
        .corner_radius(CornerRadius::same(5)),
    )
}
