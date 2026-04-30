//! avr_asm highlighter layoutjob full_span egui_cursor_safe
//! colours: mnemonic accent, reg white, imm literal, label cyan, comment dim

use eframe::egui::{
    text::{LayoutJob, TextFormat},
    Color32, FontId,
};

use crate::theme;

/// highlight_avr text→layout_job wrap_width caller_side
pub fn highlight_avr(text: &str, font_id: &FontId) -> LayoutJob {
    let mut job = LayoutJob::default();

    let mut start = 0usize;
    loop {
        match text[start..].find('\n') {
            Some(rel) => {
                let end = start + rel;
                hl_line(&mut job, &text[start..end], font_id);
                push(&mut job, "\n", theme::text_primary(), font_id);
                start = end + 1;
            }
            None => {
                hl_line(&mut job, &text[start..], font_id);
                break;
            }
        }
    }

    job
}

// line_level_highlighter

fn hl_line(job: &mut LayoutJob, line: &str, font: &FontId) {
    if line.is_empty() {
        return;
    }

    let mnemonic = theme::accent();
    let comment = theme::accent_dim();
    let number = theme::literal_num();
    let label = theme::label_cyan();
    let reg = theme::text_primary();
    let punct = theme::syntax_punct();
    let ws_col = theme::syntax_ws();
    let plain = theme::text_primary();

    let b = line.as_bytes();
    let len = line.len();
    let mut c = 0usize; // byte cursor

    // lead_ws
    let ws = non_ws(b, c);
    if ws > c {
        push(job, &line[c..ws], ws_col, font);
        c = ws;
    }
    if c >= len {
        return;
    }

    // asm_comment
    if b[c] == b';' {
        push(job, &line[c..], comment, font);
        return;
    }

    // first_word label_or_mnem
    let w_end = word_end(b, c, len);
    let word = &line[c..w_end];

    if b.get(w_end) == Some(&b':') {
        // label_def_with_colon
        push(job, &line[c..w_end + 1], label, font);
        c = w_end + 1;

        // ws_after_label
        let ws2 = non_ws(b, c);
        if ws2 > c {
            push(job, &line[c..ws2], ws_col, font);
            c = ws2;
        }
        if c >= len {
            return;
        }
        if b[c] == b';' {
            push(job, &line[c..], comment, font);
            return;
        }

        // mnem_same_line
        let m_end = word_end(b, c, len);
        push(job, &line[c..m_end], mnemonic, font);
        c = m_end;
    } else {
        push(job, word, mnemonic, font);
        c = w_end;
    }

    // operands_rest
    while c < len {
        match b[c] {
            b';' => {
                push(job, &line[c..], comment, font);
                return;
            }
            b' ' | b'\t' => {
                let ws = non_ws(b, c);
                push(job, &line[c..ws], ws_col, font);
                c = ws;
            }
            b',' => {
                push(job, ",", punct, font);
                c += 1;
            }
            ch if is_word_start(ch) => {
                let tok_end = word_end(b, c, len);
                let tok = &line[c..tok_end];
                let color = if is_register(tok) {
                    reg
                } else if is_number(tok) {
                    number
                } else {
                    plain
                };
                push(job, tok, color, font);
                c = tok_end;
            }
            _ => {
                // other_utf8_step
                let ch_len = line[c..].chars().next().map_or(1, |ch| ch.len_utf8());
                push(job, &line[c..c + ch_len], plain, font);
                c += ch_len;
            }
        }
    }
}

// helpers

/// non_ws cursor
fn non_ws(b: &[u8], mut i: usize) -> usize {
    while i < b.len() && (b[i] == b' ' || b[i] == b'\t') {
        i += 1;
    }
    i
}

/// word_end alnum_underscore_dot
fn word_end(b: &[u8], mut i: usize, len: usize) -> usize {
    while i < len && is_word_char(b[i]) {
        i += 1;
    }
    i
}

fn is_word_start(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || b == b'.'
}

fn is_word_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || b == b'.'
}

/// is_register r0_r31_xyz
fn is_register(word: &str) -> bool {
    let u = word.to_ascii_uppercase();
    if let Some(digits) = u.strip_prefix('R') {
        if let Ok(n) = digits.parse::<u32>() {
            return n <= 31;
        }
    }
    matches!(
        u.as_str(),
        "X" | "Y" | "Z" | "XL" | "XH" | "YL" | "YH" | "ZL" | "ZH"
    )
}

/// is_number lit_shapes
fn is_number(word: &str) -> bool {
    if word.is_empty() {
        return false;
    }
    let s = word.trim_start_matches('-');
    if s.is_empty() {
        return false;
    }
    s.starts_with("0x")
        || s.starts_with("0X")
        || s.starts_with("0b")
        || s.starts_with("0B")
        || s.chars().all(|c| c.is_ascii_digit())
}

/// push coloured span
fn push(job: &mut LayoutJob, text: &str, color: Color32, font_id: &FontId) {
    if text.is_empty() {
        return;
    }
    job.append(
        text,
        0.0,
        TextFormat {
            font_id: font_id.clone(),
            color,
            ..Default::default()
        },
    );
}
