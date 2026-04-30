//! editor rel_line_nums neovim_style avr_syntax

use eframe::egui::{
    self,
    gui_zoom::kb_shortcuts,
    text::{CCursor, CCursorRange, CursorRange, LayoutJob, TextFormat},
    Align2, Color32, FontFamily, FontId, Id, Key, Margin, Modifiers, Order, Pos2, Rect, RichText,
    ScrollArea, TextEdit, Ui, Vec2,
    pos2,
};

use crate::modal_chrome::{
    modal_btn_secondary, modal_single_line_edit_with_id, search_bar_frame,
};
use crate::syntax::highlight_avr;
use crate::theme;

pub struct SearchBar {
    pub visible:             bool,
    pub query:               String,
    prev_query:              String,
    pub matches:             Vec<usize>,
    pub current:             usize,
    needs_focus:             bool,
    pub next_scroll:         Option<f32>,
    pending_cursor:          bool,
    select_all_on_focus:     bool,
    id:                      Id,
}

impl SearchBar {
    fn new(parent_id: Id) -> Self {
        Self {
            visible:             false,
            query:               String::new(),
            prev_query:          String::new(),
            matches:             Vec::new(),
            current:             0,
            needs_focus:         false,
            next_scroll:         None,
            pending_cursor:      false,
            select_all_on_focus: false,
            id:                  parent_id.with("search_input"),
        }
    }

    pub fn open(&mut self) {
        self.visible             = true;
        self.needs_focus         = true;
        self.select_all_on_focus = true;
    }

    pub fn close(&mut self) {
        self.visible        = false;
        self.query.clear();
        self.prev_query.clear();
        self.matches.clear();
        self.current        = 0;
        self.pending_cursor = false;
    }

    pub fn rebuild(&mut self, source: &str) {
        if self.query == self.prev_query { return; }
        self.prev_query = self.query.clone();
        self.matches.clear();
        self.current = 0;
        if self.query.is_empty() { return; }
        let q_lo: Vec<char> = self.query.to_lowercase().chars().collect();
        let s_lo: Vec<char> = source.to_lowercase().chars().collect();
        let m = q_lo.len();
        let n = s_lo.len();
        let mut ci = 0usize;
        while ci + m <= n {
            if s_lo[ci..ci + m] == q_lo[..] {
                self.matches.push(ci);
                ci += m;
            } else {
                ci += 1;
            }
        }
    }

    pub fn navigate(&mut self, delta: i32, source: &str, row_h: f32) {
        if self.matches.is_empty() { return; }
        let n = self.matches.len();
        self.current = ((self.current as i64 + delta as i64).rem_euclid(n as i64)) as usize;
        self.pending_cursor = true;
        self.schedule_scroll(source, row_h);
    }

    fn schedule_scroll(&mut self, source: &str, row_h: f32) {
        if let Some(&ci) = self.matches.get(self.current) {
            let byte = char_to_byte(source, ci);
            let line = source[..byte].chars().filter(|&c| c == '\n').count();
            self.next_scroll = Some((line as f32 * row_h).max(0.0));
        }
    }
}

fn char_to_byte(s: &str, ci: usize) -> usize {
    s.char_indices().nth(ci).map(|(b, _)| b).unwrap_or(s.len())
}

fn char_slice(s: &str, start_c: usize, end_c: usize) -> &str {
    let start_b = char_to_byte(s, start_c);
    let end_b   = char_to_byte(s, end_c);
    &s[start_b..end_b]
}

#[inline]
fn gutter_row_center_y(galley_pos_y: f32, row_min_y: f32, row_max_y: f32, row_h: f32) -> f32 {
    let bottom = row_max_y.max(row_min_y + row_h);
    galley_pos_y + (row_min_y + bottom) * 0.5
}

fn editor_line_vertical_span(
    i: usize,
    nlines: usize,
    line_ys: &[f32],
    galley_top: f32,
    content_bottom: f32,
) -> (f32, f32) {
    let top = if i == 0 {
        galley_top
    } else {
        (line_ys[i - 1] + line_ys[i]) * 0.5
    };
    let bottom = if i + 1 < nlines {
        (line_ys[i] + line_ys[i + 1]) * 0.5
    } else {
        content_bottom
    };
    (top, bottom)
}

fn line_char_range_at_cursor(text: &str, cursor_c: usize) -> (usize, usize) {
    let mut line_start = 0usize;
    let mut pos        = 0usize;
    for ch in text.chars() {
        if ch == '\n' {
            if cursor_c <= pos {
                return (line_start, pos);
            }
            line_start = pos + 1;
        }
        pos += 1;
    }
    (line_start, pos)
}

fn leading_tab_prefix(line: &str) -> &str {
    let n = line.as_bytes().iter().take_while(|&&b| b == b'\t').count();
    &line[..n]
}

fn try_smart_enter_insert(source: &mut String, cursor_c: usize) -> Option<usize> {
    let (line_start, line_end) = line_char_range_at_cursor(source, cursor_c);
    let full_line    = char_slice(source, line_start, line_end);
    let leading_tabs = leading_tab_prefix(full_line);
    if leading_tabs.is_empty() { return None; }
    let rest = full_line.strip_prefix(leading_tabs).unwrap_or("");
    let insert = if rest.trim().is_empty() {
        "\n".to_string()
    } else {
        format!("\n{}", leading_tabs)
    };
    let insert_len = insert.chars().count();
    let b = char_to_byte(source, cursor_c);
    source.insert_str(b, &insert);
    Some(cursor_c + insert_len)
}

fn apply_highlight(
    job:        &mut egui::text::LayoutJob,
    byte_start: usize,
    byte_end:   usize,
    bg:         Color32,
) {
    let mut out = Vec::with_capacity(job.sections.len() + 2);
    for sec in job.sections.drain(..) {
        let ss = sec.byte_range.start;
        let se = sec.byte_range.end;
        if se <= byte_start || ss >= byte_end {
            out.push(sec);
        } else {
            if ss < byte_start {
                let mut before = sec.clone();
                before.byte_range = ss..byte_start;
                out.push(before);
            }
            let mut mid = sec.clone();
            mid.byte_range        = byte_start.max(ss)..byte_end.min(se);
            mid.format.background = bg;
            out.push(mid);
            if se > byte_end {
                let mut after = sec.clone();
                after.byte_range = byte_end..se;
                out.push(after);
            }
        }
    }
    job.sections = out;
}

pub struct TextEditor {
    source:       String,
    saved_source: String,
    id:           Id,
    needs_focus:  bool,
    cursor_line:  usize,
    board_inline_accept_ok: bool,
    monospace_px: f32,
    pub search:   SearchBar,
    line_y_cache: Vec<f32>,
    last_content_bottom_cache: f32,
    zebra_anchor: Option<Pos2>,
}

impl TextEditor {
    pub fn new(id: Id) -> Self {
        let search = SearchBar::new(id);
        Self {
            source:       String::new(),
            saved_source: String::new(),
            id,
            needs_focus:  false,
            cursor_line:  0,
            board_inline_accept_ok: false,
            monospace_px: 14.0,
            search,
            line_y_cache:              Vec::new(),
            last_content_bottom_cache: 0.0,
            zebra_anchor:              None,
        }
    }

    pub fn reset_for_session(&mut self) {
        self.cursor_line = 0;
        self.zebra_anchor = None;
        self.focus_next_frame();
    }

    pub fn set_source(&mut self, source: String) {
        self.saved_source              = source.clone();
        self.source                    = source;
        self.cursor_line               = 0;
        self.line_y_cache.clear();
        self.last_content_bottom_cache = 0.0;
        self.zebra_anchor = None;
    }

    pub fn source(&self) -> &str { &self.source }
    pub fn is_dirty(&self) -> bool { self.source != self.saved_source }
    pub fn mark_saved(&mut self) { self.saved_source = self.source.clone(); }
    pub fn discard_unsaved_changes(&mut self) { self.source = self.saved_source.clone(); }
    pub fn focus_next_frame(&mut self) { self.needs_focus = true; }

    pub fn request_initial_focus(&mut self, ctx: &egui::Context) {
        if self.needs_focus {
            ctx.memory_mut(|mem| mem.request_focus(self.id));
            self.needs_focus = false;
        }
    }

    pub fn text_edit_id(&self) -> Id { self.id }

    pub fn board_inline_accept_ok(&self) -> bool { self.board_inline_accept_ok }

    pub fn apply_editor_zoom_keyboard(&mut self, ctx: &egui::Context) {
        if !ctx.memory(|m| m.has_focus(self.id)) { return; }
        if ctx.input_mut(|i| i.consume_shortcut(&kb_shortcuts::ZOOM_RESET)) {
            self.monospace_px = 14.0;
            ctx.request_repaint();
            return;
        }
        if ctx.input_mut(|i| i.consume_shortcut(&kb_shortcuts::ZOOM_IN))
            || ctx.input_mut(|i| i.consume_shortcut(&kb_shortcuts::ZOOM_IN_SECONDARY))
        {
            self.monospace_px = (self.monospace_px + 1.0).min(28.0);
            ctx.request_repaint();
            return;
        }
        if ctx.input_mut(|i| i.consume_shortcut(&kb_shortcuts::ZOOM_OUT)) {
            self.monospace_px = (self.monospace_px - 1.0).max(8.0);
            ctx.request_repaint();
        }
    }

    pub fn apply_board_inline_completion(&mut self, ctx: &egui::Context) {
        let lines    = text_lines(&self.source);
        let line_idx = self.cursor_line.min(lines.len().saturating_sub(1));
        let Some(line) = lines.get(line_idx) else { return; };
        let Some((indent, partial)) = parse_dot_board_line(line) else { return; };
        let Some(suffix) = board_ghost_suffix(partial) else { return; };
        let p        = partial.trim();
        let chip     = format!("{p}{suffix}");
        let new_line = format!("{indent}.board {chip}");
        replace_line_in_source(&mut self.source, line_idx, &new_line);
        let eol = line_end_char_index(&self.source, line_idx);
        if let Some(mut ts) = TextEdit::load_state(ctx, self.id) {
            ts.cursor.set_char_range(Some(CCursorRange::one(CCursor::new(eol))));
            TextEdit::store_state(ctx, self.id, ts);
        }
        ctx.request_repaint();
    }

    pub fn show(&mut self, ui: &mut Ui, show_ghost_hint: bool, text_back: Color32) {
        let cmd_f = ui.input_mut(|i| {
            i.consume_shortcut(&egui::KeyboardShortcut::new(
                if cfg!(target_os = "macos") { Modifiers::MAC_CMD } else { Modifiers::CTRL },
                Key::F,
            ))
        });
        if cmd_f { self.search.open(); }

        if self.search.visible {
            let search_focused = ui.ctx().memory(|m| m.has_focus(self.search.id));
            if search_focused {
                if ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape)) {
                    self.search.close();
                    self.focus_next_frame();
                }
            }
        }

        let font_id = FontId::new(self.monospace_px, FontFamily::Monospace);
        let row_h   = ui.fonts(|f| f.row_height(&font_id));

        if self.search.visible && ui.ctx().memory(|m| m.has_focus(self.search.id)) {
            if ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Enter)) {
                let snap = self.source.clone();
                self.search.navigate(1, &snap, row_h);
            }
        }

        if ui.ctx().memory(|m| m.has_focus(self.id)) {
            let enter_no_shift = ui.input(|i| i.key_pressed(Key::Enter) && !i.modifiers.shift);
            if enter_no_shift {
                if let Some(mut ts) = TextEdit::load_state(ui.ctx(), self.id) {
                    if let Some(ccr) = ts.cursor.char_range() {
                        let collapsed = ccr.primary.index == ccr.secondary.index
                            && ccr.primary.prefer_next_row == ccr.secondary.prefer_next_row;
                        if collapsed {
                            if let Some(new_c) =
                                try_smart_enter_insert(&mut self.source, ccr.primary.index)
                            {
                                ts.cursor.set_char_range(Some(CCursorRange::one(
                                    CCursor::new(new_c),
                                )));
                                TextEdit::store_state(ui.ctx(), self.id, ts);
                                ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Enter));
                            }
                        }
                    }
                }
            }
        }

        let n          = line_count(&self.source);
        let digit_cols = (n.max(1).ilog10() + 1).max(3) as usize;
        let gutter_w   = ui.fonts(|f| f.glyph_width(&font_id, '0') * digit_cols as f32 + 14.0);

        let editor_rect = ui.available_rect_before_wrap();
        let bg_resp     = ui.interact(editor_rect, self.id.with("bg"), egui::Sense::click());

        if self.search.visible {
            let snap = self.source.clone();
            self.search.rebuild(&snap);
        }

        if self.search.pending_cursor && !self.search.matches.is_empty() {
            let ci   = self.search.matches[self.search.current];
            let qlen = self.search.query.chars().count();
            if let Some(mut ts) = egui::TextEdit::load_state(ui.ctx(), self.id) {
                ts.cursor.set_char_range(Some(egui::text::CCursorRange::two(
                    CCursor { index: ci,        prefer_next_row: false },
                    CCursor { index: ci + qlen, prefer_next_row: false },
                )));
                egui::TextEdit::store_state(ui.ctx(), self.id, ts);
            }
            self.search.pending_cursor = false;
        }

        let search_vis     = self.search.visible;
        let search_query   = if search_vis { self.search.query.clone() } else { String::new() };
        let search_current = self.search.current;
        let font_id_cap    = font_id.clone();

        let mut layouter = move |ui: &egui::Ui, text: &str, wrap_width: f32| {
            let mut job = highlight_avr(text, &font_id_cap);
            if !search_query.is_empty() {
                let q_lo: Vec<char> = search_query.to_lowercase().chars().collect();
                let t_lo: Vec<char> = text.to_lowercase().chars().collect();
                let qm = q_lo.len();
                let tn = t_lo.len();
                let mut ci        = 0usize;
                let mut match_idx = 0usize;
                while ci + qm <= tn {
                    if t_lo[ci..ci + qm] == q_lo[..] {
                        let bs  = char_to_byte(text, ci);
                        let be  = char_to_byte(text, ci + qm);
                        let col = if match_idx == search_current {
                            theme::match_cur()
                        } else {
                            theme::match_dim()
                        };
                        apply_highlight(&mut job, bs, be, col);
                        match_idx += 1;
                        ci += qm;
                    } else {
                        ci += 1;
                    }
                }
            }
            job.wrap.max_width = wrap_width;
            ui.fonts(|f| f.layout_job(job))
        };

        let scroll_offset = self.search.next_scroll.take();
        let mut sa = ScrollArea::vertical()
            .id_salt("editor_scroll")
            .auto_shrink([false, false]);
        if let Some(y) = scroll_offset {
            sa = sa.scroll_offset(Vec2::new(0.0, y));
        }

        sa.show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.horizontal_top(|ui| {
                ui.add_space(gutter_w);

                let lines   = text_lines(&self.source);
                let nlines  = lines.len();
                let clip = ui.clip_rect();

                if let Some(prev_gp) = self.zebra_anchor {
                    let wallpaper_like = text_back.a() < 10;
                    let mix_base = if wallpaper_like {
                        theme::panel_deep()
                    } else {
                        text_back
                    };
                    let base_fill   = mix_base;
                    let stripe_fill = theme::editor_zebra_alt_fill(mix_base);

                    let line_ys_zebra: Vec<f32> = if self.line_y_cache.len() == nlines {
                        self.line_y_cache.clone()
                    } else {
                        (0..nlines)
                            .map(|i| prev_gp.y + (i as f32) * row_h + row_h * 0.5)
                            .collect()
                    };
                    let content_bottom_z = if self.line_y_cache.len() == nlines {
                        self.last_content_bottom_cache
                    } else {
                        prev_gp.y + (nlines.max(1) as f32) * row_h
                    };

                    let painter_z = ui.painter();
                    for i in 0..nlines {
                        let fill = if i % 2 == 0 { base_fill } else { stripe_fill };
                        let (mut top, mut bottom) = editor_line_vertical_span(
                            i,
                            nlines,
                            &line_ys_zebra,
                            prev_gp.y,
                            content_bottom_z,
                        );
                        if !top.is_finite() || !bottom.is_finite() {
                            continue;
                        }
                        top = top.min(bottom);
                        bottom = bottom.max(top + 1.0);
                        let stripe_rect = Rect::from_min_max(
                            pos2(clip.left(), top),
                            pos2(clip.right(), bottom),
                        )
                        .intersect(clip);
                        if stripe_rect.width() > 0.0 && stripe_rect.height() > 0.0 {
                            painter_z.rect_filled(stripe_rect, 0.0, fill);
                        }
                    }

                    let mut top = content_bottom_z;
                    let mut vid = nlines;
                    while top < clip.bottom() {
                        let fill = if vid % 2 == 0 {
                            base_fill
                        } else {
                            stripe_fill
                        };
                        let bottom_y = (top + row_h).min(clip.bottom());
                        let phantom_rect = Rect::from_min_max(
                            pos2(clip.left(), top),
                            pos2(clip.right(), bottom_y),
                        )
                        .intersect(clip);
                        if phantom_rect.width() > 0.0 && phantom_rect.height() > 0.0 {
                            painter_z.rect_filled(phantom_rect, 0.0, fill);
                        }
                        top += row_h;
                        vid += 1;
                    }
                }

                let output = TextEdit::multiline(&mut self.source)
                    .id(self.id)
                    .frame(false)
                    .code_editor()
                    .margin(Margin::ZERO)
                    .background_color(Color32::TRANSPARENT)
                    .desired_width(ui.available_width())
                    .desired_rows(1)
                    .layouter(&mut layouter)
                    .show(ui);

                let galley      = output.galley.clone();
                let galley_pos  = output.galley_pos;
                self.zebra_anchor = Some(galley_pos);

                let cursor_range: Option<CursorRange> = output.cursor_range;

                self.board_inline_accept_ok = false;

                if let Some(ref cr) = cursor_range {
                    self.cursor_line = cr.primary.pcursor.paragraph;
                } else if ui.ctx().memory(|m| m.has_focus(self.id)) {
                    if let Some(ts) = TextEdit::load_state(ui.ctx(), self.id) {
                        if let Some(ccr) = ts.cursor.char_range() {
                            self.cursor_line =
                                line_index_for_char_index(&self.source, ccr.primary.index);
                        }
                    }
                }

                if let Some(ref cr) = cursor_range {
                    if cr.is_empty() {
                        let line_idx = cr.primary.pcursor.paragraph;
                        let cursor_c = cr.primary.ccursor.index;
                        if let Some(line) = text_lines(&self.source).get(line_idx) {
                            if parse_dot_board_line(line)
                                .and_then(|(_, p)| board_ghost_suffix(p))
                                .is_some()
                                && cursor_at_line_end(&self.source, line_idx, cursor_c)
                            {
                                self.board_inline_accept_ok = true;
                            }
                        }
                    }
                }

                let current_line = self.cursor_line.min(nlines.saturating_sub(1));
                let gutter_right = galley_pos.x - 4.0;
                let tilde_color  = theme::editor_placeholder();

                let mut new_line_ys: Vec<f32>  = Vec::with_capacity(nlines);
                let mut new_last_bottom: f32   = galley_pos.y + row_h; // safe minimum
                {
                    let mut p = 0usize;
                    let mut first_row_of_p: Option<f32> = None; // screen Y center of first row
                    let mut max_y_of_p: f32 = 0.0;

                    for row in &galley.rows {
                        if p < nlines {
                            if first_row_of_p.is_none() {
                                first_row_of_p = Some(gutter_row_center_y(
                                    galley_pos.y,
                                    row.min_y(),
                                    row.max_y(),
                                    row_h,
                                ));
                            }
                            max_y_of_p = row.max_y();
                        }
                        if row.ends_with_newline {
                            if let Some(yc) = first_row_of_p.take() {
                                new_line_ys.push(yc);
                            } else if p < nlines {
                                let fallback = galley_pos.y
                                    + (p as f32) * row_h
                                    + row_h * 0.5;
                                new_line_ys.push(fallback);
                            }
                            new_last_bottom = new_last_bottom.max(galley_pos.y + max_y_of_p);
                            p += 1;
                            first_row_of_p = None;
                            max_y_of_p     = 0.0;
                        }
                    }
                    if p < nlines {
                        if let Some(yc) = first_row_of_p {
                            new_line_ys.push(yc);
                            new_last_bottom = new_last_bottom
                                .max(galley_pos.y + max_y_of_p);
                        } else {
                            new_line_ys.push(galley_pos.y + row_h * 0.5);
                            new_last_bottom = galley_pos.y + row_h;
                        }
                    }
                }

                while new_line_ys.len() < nlines {
                    let idx = new_line_ys.len();
                    new_line_ys.push(galley_pos.y + (idx as f32) * row_h + row_h * 0.5);
                }

                let galley_settled = new_last_bottom > galley_pos.y + row_h * 0.5;

                if galley_settled || self.line_y_cache.len() != nlines {
                    self.line_y_cache              = new_line_ys.clone();
                    self.last_content_bottom_cache = new_last_bottom;
                }

                let line_ys      = &self.line_y_cache;
                let content_bottom = self.last_content_bottom_cache;
                let painter = ui.painter();

                let tilde_start_y = content_bottom;
                let mut y = tilde_start_y;
                while y + row_h * 0.35 < clip.bottom() {
                    painter.text(
                        egui::pos2(gutter_right, y + row_h * 0.5),
                        Align2::RIGHT_CENTER,
                        "~",
                        font_id.clone(),
                        tilde_color,
                    );
                    y += row_h;
                }

                for i in 0..nlines {
                    let is_current = i == current_line;
                    let display = if is_current {
                        format!("{}", i + 1)
                    } else {
                        let dist = (i as isize - current_line as isize).unsigned_abs();
                        format!("{dist}")
                    };
                    let on_stripe = i % 2 == 1;
                    let color = if is_current {
                        theme::text_primary()
                    } else if on_stripe {
                        theme::editor_zebra_rel_line_num_on_stripe()
                    } else {
                        theme::start_green()
                    };
                    let y_mid = line_ys.get(i).copied()
                        .unwrap_or_else(|| galley_pos.y + (i as f32) * row_h + row_h * 0.5);
                    painter.text(
                        egui::pos2(gutter_right, y_mid),
                        Align2::RIGHT_CENTER,
                        display,
                        font_id.clone(),
                        color,
                    );
                }

                if show_ghost_hint && self.source.is_empty() {
                    let mut job = LayoutJob::default();
                    job.append(
                        ".board ATmega328P",
                        0.0,
                        TextFormat {
                            font_id: font_id.clone(),
                            color:   theme::editor_placeholder(),
                            italics: false,
                            ..Default::default()
                        },
                    );
                    let hint_galley = ui.fonts(|f| f.layout_job(job));
                    let pos = galley_pos + Vec2::new(4.0, 2.0);
                    painter.galley(pos, hint_galley, theme::text_primary());
                } else if let Some(ref cr) = cursor_range {
                    if cr.is_empty() {
                        let line_idx = cr.primary.pcursor.paragraph;
                        let cursor_c = cr.primary.ccursor.index;
                        if let Some(line) = text_lines(&self.source).get(line_idx) {
                            if let Some(suffix) = parse_dot_board_line(line)
                                .and_then(|(_, p)| board_ghost_suffix(p))
                            {
                                if cursor_at_line_end(&self.source, line_idx, cursor_c) {
                                    let mut job = LayoutJob::default();
                                    job.append(
                                        suffix,
                                        0.0,
                                        TextFormat {
                                            font_id: font_id.clone(),
                                            color:   theme::editor_placeholder(),
                                            italics: true,
                                            ..Default::default()
                                        },
                                    );
                                    let g   = ui.fonts(|f| f.layout_job(job));
                                    let r   = galley.pos_from_cursor(&cr.primary);
                                    let pos = galley_pos + r.min.to_vec2();
                                    painter.galley(pos, g, theme::text_primary());
                                }
                            }
                        }
                    }
                }
            });
        });

        if bg_resp.clicked() {
            ui.ctx().memory_mut(|mem| mem.request_focus(self.id));
        }

        if self.search.visible {
            let char_w  = ui.fonts(|f| f.glyph_width(&font_id, '0'));
            let q_chars = self.search.query.chars().count();
            let input_w = (char_w * (q_chars.max(2) + 2) as f32).clamp(36.0, 520.0);

            let margin_x  = 8.0_f32;
            let margin_y  = 6.0_f32;
            let snap_src  = self.source.clone();
            let find_pivot = editor_rect.right_top() + Vec2::new(-margin_x, margin_y);

            egui::Area::new(self.id.with("search_area"))
                .fixed_pos(find_pivot)
                .pivot(Align2::RIGHT_TOP)
                .constrain_to(editor_rect)
                .order(Order::Foreground)
                .interactable(true)
                .show(ui.ctx(), |ui| {
                    search_bar_frame().show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing = Vec2::new(10.0, 0.0);
                            ui.label(
                                RichText::new("Find")
                                    .monospace()
                                    .size(11.0)
                                    .color(theme::accent_dim()),
                            );

                            let query_resp = modal_single_line_edit_with_id(
                                ui,
                                &mut self.search.query,
                                Some(self.search.id),
                                input_w,
                            );

                            if self.search.needs_focus {
                                query_resp.request_focus();
                                self.search.needs_focus = false;
                            }

                            if self.search.select_all_on_focus
                                && ui.ctx().memory(|m| m.has_focus(self.search.id))
                            {
                                if let Some(mut ts) =
                                    TextEdit::load_state(ui.ctx(), self.search.id)
                                {
                                    let n = self.search.query.chars().count();
                                    ts.cursor.set_char_range(Some(CCursorRange::two(
                                        CCursor::new(0),
                                        CCursor::new(n),
                                    )));
                                    TextEdit::store_state(ui.ctx(), self.search.id, ts);
                                    self.search.select_all_on_focus = false;
                                }
                            }

                            ui.vertical(|ui| {
                                ui.spacing_mut().item_spacing = Vec2::new(0.0, 2.0);
                                if modal_btn_secondary(ui, "▲").clicked() {
                                    self.search.navigate(-1, &snap_src, row_h);
                                }
                                if modal_btn_secondary(ui, "▼").clicked() {
                                    self.search.navigate(1, &snap_src, row_h);
                                }
                            });

                            let count_str = if self.search.query.is_empty() {
                                "—".to_string()
                            } else if self.search.matches.is_empty() {
                                "0/0".to_string()
                            } else {
                                format!(
                                    "{}/{}",
                                    self.search.current + 1,
                                    self.search.matches.len()
                                )
                            };
                            ui.label(
                                RichText::new(count_str)
                                    .monospace()
                                    .size(11.0)
                                    .color(theme::accent_dim()),
                            );

                            if modal_btn_secondary(ui, "✕").clicked() {
                                self.search.close();
                                self.focus_next_frame();
                            }
                        });
                    });
                });
        }
    }
}

fn line_count(text: &str) -> usize {
    text.split('\n').count()
}

fn text_lines(source: &str) -> Vec<&str> {
    source.split('\n').collect()
}

fn line_index_for_char_index(source: &str, char_index: usize) -> usize {
    let n = line_count(source).max(1);
    let mut line = 0usize;
    for (i, c) in source.chars().enumerate() {
        if i >= char_index { break; }
        if c == '\n' { line += 1; }
    }
    line.min(n - 1)
}

fn parse_dot_board_line(line: &str) -> Option<(&str, &str)> {
    let indent_len = line.len().saturating_sub(line.trim_start().len());
    let indent     = &line[..indent_len];
    let rest       = line[indent_len..].trim_start();
    if rest.len() < 6 || !rest[..6].eq_ignore_ascii_case(".board") {
        return None;
    }
    Some((indent, rest[6..].trim_start()))
}

fn board_ghost_suffix(partial: &str) -> Option<&'static str> {
    let p     = partial.trim();
    if p.is_empty() { return Some("ATmega328P"); }
    let p_low = p.to_ascii_lowercase();
    let candidates = ["ATmega328P", "ATmega128A"];
    let mut suffs: Vec<&'static str> = Vec::new();
    for c in candidates {
        if c.to_ascii_lowercase().starts_with(&p_low) && c.len() > p.len() {
            suffs.push(&c[p.len()..]);
        }
    }
    match suffs.len() {
        0 => None,
        1 => Some(suffs[0]),
        2 if p.len() == 6 && p.eq_ignore_ascii_case("ATmega") => Some("328P"),
        2 => Some(suffs[0]),
        _ => None,
    }
}

fn replace_line_in_source(source: &mut String, line_idx: usize, new_line: &str) {
    let lines          = text_lines(source);
    let had_trailing_nl = source.ends_with('\n');
    let mut out        = String::new();
    for (i, l) in lines.iter().enumerate() {
        if i > 0 { out.push('\n'); }
        if i == line_idx { out.push_str(new_line); } else { out.push_str(l); }
    }
    if had_trailing_nl { out.push('\n'); }
    *source = out;
}

fn line_end_char_index(source: &str, line_idx: usize) -> usize {
    let lines  = text_lines(source);
    let mut pos = 0usize;
    for (i, l) in lines.iter().enumerate() {
        if i == line_idx { return pos + l.chars().count(); }
        pos += l.chars().count() + 1;
    }
    0
}

fn cursor_at_line_end(source: &str, line_idx: usize, cursor_char: usize) -> bool {
    let lines  = text_lines(source);
    let mut pos = 0usize;
    for (i, l) in lines.iter().enumerate() {
        if i == line_idx { return cursor_char == pos + l.chars().count(); }
        pos += l.chars().count() + 1;
    }
    false
}
