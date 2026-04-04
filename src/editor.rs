//! editor rel_line_nums neovim_style avr_syntax

use eframe::egui::{
    self, Align, Color32, Id, Layout, Margin, ScrollArea, TextEdit, TextStyle, Ui,
};

use crate::syntax::highlight_avr;
use crate::welcome::START_GREEN;

pub struct TextEditor {
    source: String,
    saved_source: String,
    id: Id,
    needs_focus: bool,
    /// cursor_line_idx texteditoutput_eom
    cursor_line: usize,
}

impl TextEditor {
    pub fn new(id: Id) -> Self {
        Self {
            source: String::new(),
            saved_source: String::new(),
            id,
            needs_focus: false,
            cursor_line: 0,
        }
    }

    pub fn reset_for_session(&mut self) {
        self.cursor_line = 0;
        self.focus_next_frame();
    }

    pub fn set_source(&mut self, source: String) {
        self.saved_source = source.clone();
        self.source = source;
        self.cursor_line = 0;
    }

    pub fn source(&self) -> &str {
        &self.source
    }

    pub fn is_dirty(&self) -> bool {
        self.source != self.saved_source
    }

    pub fn mark_saved(&mut self) {
        self.saved_source = self.source.clone();
    }

    pub fn focus_next_frame(&mut self) {
        self.needs_focus = true;
    }

    pub fn request_initial_focus(&mut self, ctx: &egui::Context) {
        if self.needs_focus {
            ctx.memory_mut(|mem| mem.request_focus(self.id));
            self.needs_focus = false;
        }
    }

    pub fn show(&mut self, ui: &mut Ui) {
        let font_id = TextStyle::Monospace.resolve(ui.style());
        let row_h = ui.fonts(|f| f.row_height(&font_id));
        let n = line_count(&self.source);

        // gutter_w max_abs_lineno
        let digit_cols = (n.max(1).ilog10() + 1).max(3) as usize;
        let gutter_w =
            ui.fonts(|f| f.glyph_width(&font_id, '0') * digit_cols as f32 + 14.0);

        // sense_behind_textedit gutter_padding_hits_first
        let area_rect = ui.available_rect_before_wrap();
        let bg = ui.interact(area_rect, self.id.with("bg"), egui::Sense::click());

        // prev_frame_cursor_line
        let current_line = self.cursor_line.min(n.saturating_sub(1));

        // font_id_clone_for_layout_cb
        let font_id_cap = font_id.clone();

        ScrollArea::vertical()
            .id_salt("editor_scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                ui.horizontal_top(|ui| {
                    // gutter_relative_line_numbers
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
                        ui.set_width(gutter_w);
                        for i in 0..n {
                            let is_current = i == current_line;
                            let display = if is_current {
                                // cur_line_abs
                                format!("{}", i + 1)
                            } else {
                                // rel_delta_lines
                                let dist = (i as isize - current_line as isize).unsigned_abs();
                                format!("{dist}")
                            };
                            let color = if is_current {
                                Color32::WHITE
                            } else {
                                START_GREEN
                            };
                            ui.allocate_ui_with_layout(
                                egui::vec2(gutter_w, row_h),
                                Layout::right_to_left(Align::Center),
                                |ui| {
                                    ui.label(
                                        egui::RichText::new(display)
                                            .font(font_id.clone())
                                            .color(color),
                                    );
                                },
                            );
                        }
                    });

                    // syntax_highlighted_textedit
                    let mut layouter = move |ui: &egui::Ui, text: &str, wrap_width: f32| {
                        let mut job = highlight_avr(text, &font_id_cap);
                        job.wrap.max_width = wrap_width;
                        ui.fonts(|f| f.layout_job(job))
                    };

                    let output = TextEdit::multiline(&mut self.source)
                        .id(self.id)
                        .frame(false)
                        .code_editor()
                        .margin(Margin::ZERO)
                        .background_color(Color32::BLACK)
                        .desired_width(ui.available_width())
                        .desired_rows(1)
                        .layouter(&mut layouter)
                        .show(ui);

                    // store_cursor_for_next_gutter
                    if let Some(cursor_range) = output.cursor_range {
                        self.cursor_line = cursor_range.primary.pcursor.paragraph;
                    }
                });
            });

        if bg.clicked() {
            ui.ctx().memory_mut(|mem| mem.request_focus(self.id));
        }
    }
}

fn line_count(text: &str) -> usize {
    if text.is_empty() {
        1
    } else {
        text.split('\n').count()
    }
}
