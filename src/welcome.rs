//! welcome_screen

use std::f32::consts::{SQRT_2, TAU};
use std::sync::{Arc, OnceLock};

use eframe::egui::text::{LayoutJob, TextFormat};
use eframe::egui::{self, Color32, FontId, Image, RichText, Stroke, Ui, Vec2};

use crate::welcome_font;

/// App-wide accent green (editor, sim, helpers, toolbar, docs, modals).
pub const START_GREEN: Color32 = Color32::from_rgb(0x0b, 0xca, 0x0b);

pub const START_GREEN_DIM: Color32 = START_GREEN;

// --- Welcome-only spec (outline / glow); fill matches [`START_GREEN`] ---

pub const WELCOME_FILL: Color32 = START_GREEN;
pub const WELCOME_OUTLINE: Color32 = Color32::from_rgb(0x21, 0x3f, 0x00);

const OUTLINE_SOFT_X: f32 = 15.51;
const OUTLINE_SOFT_Y: f32 = 14.39;
const OUTLINE_GLOW: f32 = 0.355;
const OUTLINE_THICK_REL: f32 = 0.0888;

const FILL_SOFT_X: f32 = 2.99;
const FILL_SOFT_Y: f32 = 0.93;
const FILL_GLOW: f32 = 0.178;

/// Maps design softness numbers to point offsets (~proportional to cap height).
const SOFT_TO_PT: f32 = 1.0 / 72.0;

pub const WELCOME_SIZE_PCT: u8 = 68;

const MARGIN: f32 = 44.0;

const TITLE_REF_PX: f32 = 54.0;

fn size_factor() -> f32 {
    (WELCOME_SIZE_PCT.clamp(1, 100) as f32) / 100.0
}

fn layout_welcome_galley(ui: &Ui, text: &str, size: f32, spacing: f32) -> Arc<egui::Galley> {
    let job = LayoutJob::single_section(
        text.to_owned(),
        TextFormat {
            font_id: FontId::new(size, welcome_font::family()),
            color: WELCOME_FILL,
            extra_letter_spacing: spacing,
            ..Default::default()
        },
    );
    ui.fonts(|f| f.layout_job(job))
}

fn measure_title_line(ui: &Ui, text: &str, size: f32, spacing: f32) -> f32 {
    layout_welcome_galley(ui, text, size, spacing).size().x
}

fn oct_offsets(r: f32) -> [(f32, f32); 8] {
    let s = SQRT_2.recip() * r;
    [
        (-r, 0.0),
        (r, 0.0),
        (0.0, -r),
        (0.0, r),
        (-s, -s),
        (s, -s),
        (-s, s),
        (s, s),
    ]
}

/// Layered draws approximating anisotropic outer glow + thick outline + inner fill glow.
fn paint_welcome_text_glow(ui: &Ui, galley: Arc<egui::Galley>, pos: egui::Pos2, font_size: f32) {
    let painter = ui.painter();

    let sx_o = font_size * OUTLINE_SOFT_X * SOFT_TO_PT;
    let sy_o = font_size * OUTLINE_SOFT_Y * SOFT_TO_PT;
    let sx_f = font_size * FILL_SOFT_X * SOFT_TO_PT;
    let sy_f = font_size * FILL_SOFT_Y * SOFT_TO_PT;
    let thick = (font_size * OUTLINE_THICK_REL).max(0.5);

    let outline_glow_alpha = (OUTLINE_GLOW * 255.0) as u8;
    let fill_glow_alpha = (FILL_GLOW * 255.0) as u8;

    const RINGS: i32 = 5;
    const SPOKES: i32 = 10;

    for ring in 1..=RINGS {
        let t = ring as f32 / RINGS as f32;
        let rx = sx_o * t;
        let ry = sy_o * t;
        let a = (outline_glow_alpha as f32 * (1.0 - t * 0.35)) / 255.0;
        for k in 0..SPOKES {
            let ang = TAU * k as f32 / SPOKES as f32;
            let dx = rx * ang.cos();
            let dy = ry * ang.sin();
            painter.galley_with_override_text_color(
                pos + egui::vec2(dx, dy),
                galley.clone(),
                WELCOME_OUTLINE.gamma_multiply(a),
            );
        }
    }

    for &(dx, dy) in &oct_offsets(thick) {
        painter.galley_with_override_text_color(
            pos + egui::vec2(dx, dy),
            galley.clone(),
            WELCOME_OUTLINE,
        );
    }
    let inner = thick * 0.52;
    for &(dx, dy) in &oct_offsets(inner) {
        painter.galley_with_override_text_color(
            pos + egui::vec2(dx, dy),
            galley.clone(),
            WELCOME_OUTLINE.gamma_multiply(0.72),
        );
    }

    for k in 0..8 {
        let ang = TAU * k as f32 / 8.0;
        let dx = sx_f * 0.55 * ang.cos();
        let dy = sy_f * 0.55 * ang.sin();
        painter.galley_with_override_text_color(
            pos + egui::vec2(dx, dy),
            galley.clone(),
            WELCOME_FILL.gamma_multiply(fill_glow_alpha as f32 / 255.0),
        );
    }

    painter.galley_with_override_text_color(pos, galley, WELCOME_FILL);
}

fn centered_welcome_line(ui: &mut Ui, text: &str, font_size: f32, letter_sp: f32) {
    let galley = layout_welcome_galley(ui, text, font_size, letter_sp);
    let size = galley.size();
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        ui.add_space(((ui.available_width() - size.x) * 0.5).max(0.0));
        let (_, rect) = ui.allocate_space(size);
        paint_welcome_text_glow(ui, galley, rect.min, font_size);
    });
}

fn banner_pixel_size() -> (u32, u32) {
    static DIMS: OnceLock<(u32, u32)> = OnceLock::new();
    *DIMS.get_or_init(|| {
        let bytes = include_bytes!("../assets/images/lain_banner.png");
        let img = image::load_from_memory(bytes).expect(
            "assets/images/lain_banner.png should be valid image bytes (PNG or JPEG, etc.)",
        );
        (img.width(), img.height())
    })
}

fn banner_size_for_width(w: f32) -> Vec2 {
    let (pw, ph) = banner_pixel_size();
    Vec2::new(w, w * (ph as f32 / pw as f32))
}

fn clamp_banner_height(size: Vec2, max_h: f32) -> Vec2 {
    if max_h <= 0.0 || size.y <= max_h {
        return size;
    }
    let s = max_h / size.y;
    Vec2::new(size.x * s, max_h)
}

struct WelcomeStartButton {
    pub response: egui::Response,
    /// Label bounds (for sizing the cross-out over the other row).
    pub text_rect: egui::Rect,
    pub font_px: f32,
}

/// Same glow/outline as titles.
fn welcome_start_button(ui: &mut Ui, label: &str, font_px: f32) -> WelcomeStartButton {
    let galley = layout_welcome_galley(ui, label, font_px, 0.0);
    let text_size = galley.size();
    let pad = egui::vec2(10.0, 6.0);
    let hit_size = text_size + pad * 2.0;

    let mut out = None;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        ui.add_space(((ui.available_width() - hit_size.x) * 0.5).max(0.0));
        let (rect, response) = ui.allocate_exact_size(hit_size, egui::Sense::click());
        let text_rect = egui::Rect::from_center_size(rect.center(), text_size);
        paint_welcome_text_glow(ui, galley, text_rect.min, font_px);
        out = Some(WelcomeStartButton {
            response,
            text_rect,
            font_px,
        });
    });
    out.expect("horizontal always runs")
}

#[derive(Clone, Default)]
struct WelcomeCrossAnim {
    /// 0 = none, 1 = open hovered (cross create), 2 = create hovered (cross open)
    mode: u8,
    phase: u8,
    stroke0_start: f32,
    stroke1_start: f32,
}

#[inline]
fn ease_out_cubic(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    1.0 - (1.0 - t).powi(3)
}

/// Screen-space axis-aligned blur for one straight segment `from` → `to`.
fn paint_line_axis_blur(
    painter: &egui::Painter,
    from: egui::Pos2,
    to: egui::Pos2,
    color: Color32,
    stroke_w: f32,
    soft_x: f32,
    soft_y: f32,
    peak_alpha: f32,
    grid: i32,
) {
    let g = grid.max(2);
    for ix in 0..g {
        for iy in 0..g {
            let nx = (ix as f32 / (g - 1) as f32) * 2.0 - 1.0;
            let ny = (iy as f32 / (g - 1) as f32) * 2.0 - 1.0;
            let ell = (nx * nx + ny * ny).min(1.0);
            let a = (1.0 - ell).powf(2.35) * peak_alpha;
            if a < 0.0025 {
                continue;
            }
            let off = egui::vec2(nx * soft_x, ny * soft_y);
            painter.line_segment(
                [from + off, to + off],
                Stroke::new(stroke_w, color.gamma_multiply(a)),
            );
        }
    }
}

/// One diagonal: grows from **midpoint of that line** toward both corners (`from` and `to`) as `progress` 0→1.
fn paint_welcome_x_straight_pencil(
    painter: &egui::Painter,
    from: egui::Pos2,
    to: egui::Pos2,
    font_px: f32,
    _seed: u32,
    progress: f32,
) {
    let progress = progress.clamp(0.0, 1.0);
    if progress <= 0.0 {
        return;
    }

    let mid = from.lerp(to, 0.5);
    let tip_a = mid.lerp(from, progress);
    let tip_b = mid.lerp(to, progress);

    let soft_x = font_px * 2.25;
    let soft_y = font_px * 1.92;
    let blur_w = (font_px * OUTLINE_THICK_REL * 1.45).max(0.85);

    const BLUR_GRID: i32 = 13;
    const X_STROKE_ALPHA: f32 = 0.68;

    let border_w = (font_px * 0.168).max(2.4);
    let core_w = (border_w * 0.44).max(font_px * 0.068);

    let paint_half = |a: egui::Pos2, b: egui::Pos2| {
        if (b - a).length() < 1e-4 {
            return;
        }
        paint_line_axis_blur(
            painter,
            a,
            b,
            Color32::BLACK,
            blur_w * 1.2,
            soft_x,
            soft_y,
            0.052 * X_STROKE_ALPHA,
            BLUR_GRID,
        );
        paint_line_axis_blur(
            painter,
            a,
            b,
            WELCOME_FILL,
            blur_w * 1.05,
            soft_x * 0.9,
            soft_y * 0.9,
            0.068 * X_STROKE_ALPHA,
            BLUR_GRID,
        );
        painter.line_segment(
            [a, b],
            Stroke::new(border_w, Color32::BLACK.gamma_multiply(X_STROKE_ALPHA)),
        );
        painter.line_segment(
            [a, b],
            Stroke::new(core_w, WELCOME_FILL.gamma_multiply(X_STROKE_ALPHA)),
        );
    };

    paint_half(mid, tip_a);
    paint_half(mid, tip_b);
}

fn cross_out_corners(text_rect: egui::Rect, font_px: f32) -> (egui::Pos2, egui::Pos2, egui::Pos2, egui::Pos2) {
    let pad_x = font_px * 0.34 + 2.75;
    let pad_y = font_px * 0.28 + 2.25;
    let r = text_rect.expand2(egui::vec2(pad_x, pad_y));
    let inset = (font_px * 0.12).max(1.0);
    let tl = r.left_top() + Vec2::splat(inset);
    let br = r.right_bottom() - Vec2::splat(inset);
    let tr = r.right_top() + Vec2::new(-inset, inset);
    let bl = r.left_bottom() + Vec2::new(inset, -inset);
    (tl, br, tr, bl)
}

/// Animated X: each arm grows from its line midpoint toward both corners; black border + X/Y blur.
fn paint_welcome_cross_out(
    ui: &Ui,
    text_rect: egui::Rect,
    font_px: f32,
    prog1: f32,
    prog2: f32,
    seed1: u32,
    seed2: u32,
) {
    let painter = ui.painter();
    let (tl, br, tr, bl) = cross_out_corners(text_rect, font_px);
    paint_welcome_x_straight_pencil(&painter, tl, br, font_px, seed1, prog1);
    paint_welcome_x_straight_pencil(&painter, tr, bl, font_px, seed2, prog2);
}

pub enum WelcomeAction {
    None,
    OpenFolder,
    CreateNew,
}

pub fn show_welcome(ui: &mut Ui) -> WelcomeAction {
    let mut action = WelcomeAction::None;

    let avail_w = ui.available_width();
    let pct = size_factor();
    let content_w = ((avail_w - 2.0 * MARGIN).max(300.0) * pct).max(200.0);

    let ref_w = measure_title_line(ui, "LAIN STUDIO", TITLE_REF_PX, 2.0).max(1.0);
    let scale = content_w / ref_w;

    let title_px = TITLE_REF_PX * scale;
    let subtitle_px = (19.0 / TITLE_REF_PX) * title_px;
    let button_px = ((15.0 / TITLE_REF_PX) * title_px).max(14.0);
    let letter_sp = 2.0 * scale;
    let sub_letter_sp = 3.0 * scale;
    let sp = |n: f32| -> f32 { n * scale };

    ui.vertical_centered(|ui| {
        ui.add_space(MARGIN);

        centered_welcome_line(ui, "LAIN STUDIO", title_px, letter_sp);
        ui.add_space(sp(12.0));
        centered_welcome_line(ui, "CHOOSE HOW TO START", subtitle_px, sub_letter_sp);
        ui.add_space(sp(36.0));

        let btn_open = welcome_start_button(ui, "Open folder…", button_px);
        if btn_open.response.clicked() {
            action = WelcomeAction::OpenFolder;
        }
        ui.add_space(sp(8.0));
        let btn_create = welcome_start_button(ui, "Create a new file…", button_px);
        if btn_create.response.clicked() {
            action = WelcomeAction::CreateNew;
        }

        const STROKE_SECS: f32 = 0.38;
        let cross_id = egui::Id::new("lainstudio_welcome_cross_anim");
        let now = ui.ctx().input(|i| i.time) as f32;

        let desired_mode: u8 = if btn_open.response.hovered() {
            1
        } else if btn_create.response.hovered() {
            2
        } else {
            0
        };

        let (prog1, prog2, animating) = ui.ctx().data_mut(|d| {
            let st = d.get_temp_mut_or_default::<WelcomeCrossAnim>(cross_id);
            if st.mode != desired_mode {
                st.mode = desired_mode;
                st.phase = 0;
                if desired_mode != 0 {
                    st.stroke0_start = now;
                }
            }

            if desired_mode == 0 {
                return (0.0f32, 0.0f32, false);
            }

            let mut p1 = 0.0f32;
            let mut p2 = 0.0f32;
            let mut need_repaint = true;

            if st.phase == 0 {
                let t = ((now - st.stroke0_start) / STROKE_SECS).clamp(0.0, 1.0);
                p1 = ease_out_cubic(t);
                if t >= 1.0 {
                    st.phase = 1;
                    st.stroke1_start = now;
                    p1 = 1.0;
                }
            }

            if st.phase >= 1 {
                p1 = 1.0;
                let t = ((now - st.stroke1_start) / STROKE_SECS).clamp(0.0, 1.0);
                p2 = ease_out_cubic(t);
                if t >= 1.0 {
                    need_repaint = false;
                }
            }

            (p1, p2, need_repaint)
        });

        if desired_mode == 1 {
            // Open hovered → cross out Create (different seeds → different rough X shape/size)
            paint_welcome_cross_out(
                ui,
                btn_create.text_rect,
                btn_create.font_px,
                prog1,
                prog2,
                0x5EED_C0u32,
                0x71CE_u32,
            );
        } else if desired_mode == 2 {
            paint_welcome_cross_out(
                ui,
                btn_open.text_rect,
                btn_open.font_px,
                prog1,
                prog2,
                0xBEE2_u32,
                0xC0DE_u32,
            );
        }

        ui.add_space(sp(28.0));
        if animating {
            ui.ctx().request_repaint();
        }

        let max_banner_h = (ui.available_height() - MARGIN).max(0.0);
        let banner_natural = banner_size_for_width(content_w);
        let banner_size = clamp_banner_height(banner_natural, max_banner_h);

        ui.add(
            Image::new(egui::include_image!("../assets/images/lain_banner.png"))
                .fit_to_exact_size(banner_size),
        );
        ui.add_space(MARGIN);
    });
    action
}

pub enum CreateProjectAction {
    None,
    PickParentFolder,
    Back,
    Submit,
}

fn title_font(size: f32) -> FontId {
    FontId::new(size, egui::FontFamily::Name(std::sync::Arc::from("lain_title")))
}

pub fn show_create_project(
    ui: &mut Ui,
    parent_dir: &Option<std::path::PathBuf>,
    name: &mut String,
    err: &Option<String>,
) -> CreateProjectAction {
    let mut action = CreateProjectAction::None;

    let avail_w = ui.available_width();
    let pct = size_factor();
    let content_w = ((avail_w - 2.0 * MARGIN).max(300.0) * pct).max(200.0);

    let ref_w = {
        let job = LayoutJob::single_section(
            "NEW PROJECT".into(),
            TextFormat {
                font_id: title_font(TITLE_REF_PX),
                color: START_GREEN,
                extra_letter_spacing: 2.0,
                ..Default::default()
            },
        );
        ui.fonts(|f| f.layout_job(job).size().x)
    }
    .max(1.0);
    let scale = content_w / ref_w;

    let heading_px = TITLE_REF_PX * scale;
    let label_px = ((12.0 / TITLE_REF_PX) * heading_px).max(13.0);
    let body_px = ((14.0 / TITLE_REF_PX) * heading_px).max(13.0);
    let button_px = ((15.0 / TITLE_REF_PX) * heading_px).max(14.0);
    let letter_sp = 2.0 * scale;
    let sp = |n: f32| -> f32 { n * scale };

    fn green_button(ui: &mut Ui, label: &str, font_px: f32) -> egui::Response {
        ui.add(
            egui::Button::new(
                RichText::new(label)
                    .color(START_GREEN)
                    .font(FontId::monospace(font_px)),
            )
            .fill(Color32::TRANSPARENT)
            .stroke(Stroke::new(1.0, START_GREEN)),
        )
    }

    ui.vertical_centered(|ui| {
        ui.add_space(MARGIN);
        ui.label(
            RichText::new("NEW PROJECT")
                .font(title_font(heading_px))
                .color(START_GREEN)
                .extra_letter_spacing(letter_sp),
        );
        ui.add_space(sp(20.0));

        ui.label(
            RichText::new("PARENT LOCATION")
                .font(title_font(label_px))
                .color(START_GREEN_DIM)
                .extra_letter_spacing(letter_sp),
        );
        let parent_label = parent_dir
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "(not chosen)".to_string());
        ui.label(
            RichText::new(parent_label)
                .monospace()
                .color(START_GREEN)
                .size(body_px),
        );
        ui.add_space(sp(6.0));
        if green_button(ui, "Choose location…", button_px).clicked() {
            action = CreateProjectAction::PickParentFolder;
        }

        ui.add_space(sp(16.0));
        ui.label(
            RichText::new("NAME")
                .font(title_font(label_px))
                .color(START_GREEN_DIM)
                .extra_letter_spacing(letter_sp),
        );
        ui.label(
            RichText::new("Creates a folder with this name and a matching .lain file inside.")
                .size(body_px * 0.85)
                .color(START_GREEN_DIM),
        );
        ui.add_space(sp(4.0));
        ui.add(
            egui::TextEdit::singleline(name)
                .desired_width((content_w).min(520.0))
                .font(FontId::monospace(body_px))
                .text_color(START_GREEN)
                .hint_text(RichText::new("my_project").color(START_GREEN_DIM)),
        );

        if let Some(msg) = err {
            ui.add_space(sp(8.0));
            ui.colored_label(Color32::from_rgb(255, 140, 140), msg);
        }

        ui.add_space(sp(24.0));
        ui.horizontal(|ui| {
            if green_button(ui, "Back", button_px).clicked() {
                action = CreateProjectAction::Back;
            }
            ui.add_space(sp(12.0));
            if green_button(ui, "Create", button_px).clicked() {
                action = CreateProjectAction::Submit;
            }
        });
        ui.add_space(MARGIN);
    });
    action
}
