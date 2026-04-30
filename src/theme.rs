//! ui palette — runtime `ThemePalette` (customizable) plus egui `Visuals` application.

use std::collections::BTreeMap;
use std::sync::Mutex;
use std::sync::OnceLock;

use eframe::egui::{
    self, emath::Rot2, pos2, style::HandleShape, AboveOrBelow, Color32, ComboBox, CornerRadius,
    FontFamily, FontId, Id, Pos2, Shadow, Stroke, TextStyle, Ui, Visuals,
};


use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Rgb([u8; 3]);

impl From<Color32> for Rgb {
    fn from(c: Color32) -> Self {
        Rgb([c.r(), c.g(), c.b()])
    }
}

impl From<Rgb> for Color32 {
    fn from(r: Rgb) -> Self {
        Color32::from_rgb(r.0[0], r.0[1], r.0[2])
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ThemePaletteFile {
    pub accent: Rgb,
    pub accent_dim: Rgb,
    pub focus: Rgb,
    pub literal_num: Rgb,
    pub label_cyan: Rgb,
    pub err_red: Rgb,
    pub dim_gray: Rgb,
    pub section: Rgb,
    pub panel_deep: Rgb,
    pub panel_mid: Rgb,
    pub panel_lift: Rgb,
    pub button_fill_strong: Rgb,
    pub disabled_panel: Rgb,
    pub editor_placeholder: Rgb,
    pub search_bg: Rgb,
    pub match_dim: Rgb,
    pub match_cur: Rgb,
    pub sim_surface: Rgb,
    pub sim_surface_lift: Rgb,
    pub sim_tab_active: Rgb,
    pub sim_border: Rgb,
    pub sim_border_bright: Rgb,
    pub sim_stop_fill: Rgb,
    pub sim_stop_border: Rgb,
    pub main_central_fill: Rgb,
    pub text_primary: Rgb,
    pub on_accent_text: Rgb,
    pub status_error: Rgb,
    pub err_red_soft: Rgb,
    pub syntax_punct: Rgb,
    pub syntax_ws: Rgb,
    pub periph_dim: Rgb,
    #[serde(default = "default_periph_pin_used_json")]
    pub periph_pin_used: Rgb,
    pub selection_bg: Rgb,
}

fn default_periph_pin_used_json() -> Rgb {
    Rgb([255, 210, 72])
}

impl From<ThemePalette> for ThemePaletteFile {
    fn from(t: ThemePalette) -> Self {
        Self {
            accent: t.accent.into(),
            accent_dim: t.accent_dim.into(),
            focus: t.focus.into(),
            literal_num: t.literal_num.into(),
            label_cyan: t.label_cyan.into(),
            err_red: t.err_red.into(),
            dim_gray: t.dim_gray.into(),
            section: t.section.into(),
            panel_deep: t.panel_deep.into(),
            panel_mid: t.panel_mid.into(),
            panel_lift: t.panel_lift.into(),
            button_fill_strong: t.button_fill_strong.into(),
            disabled_panel: t.disabled_panel.into(),
            editor_placeholder: t.editor_placeholder.into(),
            search_bg: t.search_bg.into(),
            match_dim: t.match_dim.into(),
            match_cur: t.match_cur.into(),
            sim_surface: t.sim_surface.into(),
            sim_surface_lift: t.sim_surface_lift.into(),
            sim_tab_active: t.sim_tab_active.into(),
            sim_border: t.sim_border.into(),
            sim_border_bright: t.sim_border_bright.into(),
            sim_stop_fill: t.sim_stop_fill.into(),
            sim_stop_border: t.sim_stop_border.into(),
            main_central_fill: t.main_central_fill.into(),
            text_primary: t.text_primary.into(),
            on_accent_text: t.on_accent_text.into(),
            status_error: t.status_error.into(),
            err_red_soft: t.err_red_soft.into(),
            syntax_punct: t.syntax_punct.into(),
            syntax_ws: t.syntax_ws.into(),
            periph_dim: t.periph_dim.into(),
            periph_pin_used: t.periph_pin_used.into(),
            selection_bg: t.selection_bg.into(),
        }
    }
}

impl From<ThemePaletteFile> for ThemePalette {
    fn from(f: ThemePaletteFile) -> Self {
        Self {
            accent: f.accent.into(),
            accent_dim: f.accent_dim.into(),
            focus: f.focus.into(),
            literal_num: f.literal_num.into(),
            label_cyan: f.label_cyan.into(),
            err_red: f.err_red.into(),
            dim_gray: f.dim_gray.into(),
            section: f.section.into(),
            panel_deep: f.panel_deep.into(),
            panel_mid: f.panel_mid.into(),
            panel_lift: f.panel_lift.into(),
            button_fill_strong: f.button_fill_strong.into(),
            disabled_panel: f.disabled_panel.into(),
            editor_placeholder: f.editor_placeholder.into(),
            search_bg: f.search_bg.into(),
            match_dim: f.match_dim.into(),
            match_cur: f.match_cur.into(),
            sim_surface: f.sim_surface.into(),
            sim_surface_lift: f.sim_surface_lift.into(),
            sim_tab_active: f.sim_tab_active.into(),
            sim_border: f.sim_border.into(),
            sim_border_bright: f.sim_border_bright.into(),
            sim_stop_fill: f.sim_stop_fill.into(),
            sim_stop_border: f.sim_stop_border.into(),
            main_central_fill: f.main_central_fill.into(),
            text_primary: f.text_primary.into(),
            on_accent_text: f.on_accent_text.into(),
            status_error: f.status_error.into(),
            err_red_soft: f.err_red_soft.into(),
            syntax_punct: f.syntax_punct.into(),
            syntax_ws: f.syntax_ws.into(),
            periph_dim: f.periph_dim.into(),
            periph_pin_used: f.periph_pin_used.into(),
            selection_bg: f.selection_bg.into(),
        }
    }
}

// --- runtime (Copy) ---

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct ThemePalette {
    /// primary accent: bright white with a little blue
    pub accent: Color32,
    /// secondary labels, dim strokes
    pub accent_dim: Color32,
    /// strong emphasis: ice blue
    pub focus: Color32,
    /// syntax: immediate / numeric literals
    pub literal_num: Color32,
    pub label_cyan: Color32,
    pub err_red: Color32,
    pub dim_gray: Color32,
    /// docs section headers
    pub section: Color32,
    /// deepest panel fill (main editor surround, side panels)
    pub panel_deep: Color32,
    /// mid toolbar strip, menus
    pub panel_mid: Color32,
    /// slightly lifted surface (file tabs bar, modal chrome)
    pub panel_lift: Color32,
    /// Strong button / title-bar slab
    pub button_fill_strong: Color32,
    pub disabled_panel: Color32,
    /// empty buffer watermark
    pub editor_placeholder: Color32,
    pub search_bg: Color32,
    pub match_dim: Color32,
    pub match_cur: Color32,
    // simulator
    pub sim_surface: Color32,
    pub sim_surface_lift: Color32,
    pub sim_tab_active: Color32,
    pub sim_border: Color32,
    pub sim_border_bright: Color32,
    pub sim_stop_fill: Color32,
    pub sim_stop_border: Color32,
    /// central panel / main egui void
    pub main_central_fill: Color32,
    /// default body text in egui
    pub text_primary: Color32,
    /// text on top of accent (tabs, small buttons)
    pub on_accent_text: Color32,
    /// status line error
    pub status_error: Color32,
    /// softer error in panels
    pub err_red_soft: Color32,
    /// syntax: punctuation
    pub syntax_punct: Color32,
    /// syntax: whitespace
    pub syntax_ws: Color32,
    /// peripheral rail marker (sim)
    pub periph_dim: Color32,
    /// PORTS tab: full block, dot, and legend for a peripheral-occupied output-high pin
    pub periph_pin_used: Color32,
    /// editor selection
    pub selection_bg: Color32,
}

static CURRENT: OnceLock<Mutex<ThemePalette>> = OnceLock::new();

///  default styles
impl ThemePalette {
    pub const DEFAULT: Self = Self {
        accent: Color32::from_rgb(248, 250, 255),
        accent_dim: Color32::from_rgb(125, 135, 158),
        focus: Color32::from_rgb(175, 205, 255),
        literal_num: Color32::from_rgb(195, 210, 245),
        label_cyan: Color32::from_rgb(120, 210, 255),
        err_red: Color32::from_rgb(255, 100, 100),
        dim_gray: Color32::from_rgb(65, 65, 72),
        section: Color32::from_rgb(150, 185, 220),
        panel_deep: Color32::from_rgb(4, 6, 12),
        panel_mid: Color32::from_rgb(7, 10, 18),
        panel_lift: Color32::from_rgb(12, 16, 26),
        button_fill_strong: Color32::from_rgb(14, 18, 30),
        disabled_panel: Color32::from_rgb(22, 24, 30),
        editor_placeholder: Color32::from_rgb(72, 78, 90),
        search_bg: Color32::from_rgb(4, 6, 14),
        match_dim: Color32::from_rgb(90, 110, 150),
        match_cur: Color32::from_rgb(210, 225, 255),
        sim_surface: Color32::from_rgb(9, 12, 22),
        sim_surface_lift: Color32::from_rgb(15, 19, 32),
        sim_tab_active: Color32::from_rgb(13, 17, 30),
        sim_border: Color32::from_rgb(68, 78, 98),
        sim_border_bright: Color32::from_rgb(155, 168, 198),
        sim_stop_fill: Color32::from_rgb(22, 15, 22),
        sim_stop_border: Color32::from_rgb(118, 95, 112),
        main_central_fill: Color32::BLACK,
        text_primary: Color32::WHITE,
        on_accent_text: Color32::BLACK,
        status_error: Color32::from_rgb(255, 140, 140),
        err_red_soft: Color32::from_rgb(200, 120, 120),
        syntax_punct: Color32::from_rgb(90, 90, 90),
        syntax_ws: Color32::from_rgb(55, 55, 55),
        periph_dim: Color32::from_rgb(120, 95, 40),
        periph_pin_used: Color32::from_rgb(255, 210, 72),
        selection_bg: Color32::from_rgb(55, 55, 55),
    };

    pub const ONE_DARK_PRO: Self = Self {
        accent: Color32::from_rgb(215, 218, 224),
        accent_dim: Color32::from_rgb(92, 99, 112),
        focus: Color32::from_rgb(97, 175, 239),
        literal_num: Color32::from_rgb(209, 154, 102),
        label_cyan: Color32::from_rgb(86, 182, 194),
        err_red: Color32::from_rgb(224, 108, 117),
        dim_gray: Color32::from_rgb(92, 99, 112),
        section: Color32::from_rgb(198, 120, 221),
        panel_deep: Color32::from_rgb(33, 37, 43),
        panel_mid: Color32::from_rgb(40, 44, 52),
        panel_lift: Color32::from_rgb(44, 49, 58),
        button_fill_strong: Color32::from_rgb(58, 63, 75),
        disabled_panel: Color32::from_rgb(62, 68, 81),
        editor_placeholder: Color32::from_rgb(92, 99, 112),
        search_bg: Color32::from_rgb(33, 37, 43),
        match_dim: Color32::from_rgb(75, 82, 99),
        match_cur: Color32::from_rgb(82, 139, 255),
        sim_surface: Color32::from_rgb(33, 37, 43),
        sim_surface_lift: Color32::from_rgb(44, 49, 58),
        sim_tab_active: Color32::from_rgb(40, 44, 52),
        sim_border: Color32::from_rgb(24, 26, 31),
        sim_border_bright: Color32::from_rgb(75, 82, 99),
        sim_stop_fill: Color32::from_rgb(62, 68, 81),
        sim_stop_border: Color32::from_rgb(198, 120, 221),
        main_central_fill: Color32::from_rgb(40, 44, 52),
        text_primary: Color32::from_rgb(171, 178, 191),
        on_accent_text: Color32::from_rgb(33, 37, 43),
        status_error: Color32::from_rgb(224, 108, 117),
        err_red_soft: Color32::from_rgb(190, 80, 70),
        syntax_punct: Color32::from_rgb(136, 145, 162),
        syntax_ws: Color32::from_rgb(59, 64, 72),
        periph_dim: Color32::from_rgb(209, 154, 102),
        periph_pin_used: Color32::from_rgb(152, 195, 121),
        selection_bg: Color32::from_rgb(62, 68, 81),
    };
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChromeProfile {
    Standard,
    VsCodeStyle,
}

static CHROME: OnceLock<Mutex<ChromeProfile>> = OnceLock::new();

pub fn install_chrome(profile: ChromeProfile) {
    *CHROME
        .get_or_init(|| Mutex::new(ChromeProfile::Standard))
        .lock()
        .expect("chrome mutex poisoned") = profile;
}

fn chrome() -> ChromeProfile {
    *CHROME
        .get_or_init(|| Mutex::new(ChromeProfile::Standard))
        .lock()
        .expect("chrome mutex poisoned")
}

#[must_use]
pub fn chrome_profile() -> ChromeProfile {
    chrome()
}

pub fn install(p: &ThemePalette) {
    *CURRENT
        .get_or_init(|| Mutex::new(ThemePalette::DEFAULT))
        .lock()
        .expect("theme mutex poisoned") = *p;
}

fn cur() -> ThemePalette {
    *CURRENT
        .get_or_init(|| Mutex::new(ThemePalette::DEFAULT))
        .lock()
        .expect("theme mutex poisoned")
}

pub fn accent() -> Color32 {
    cur().accent
}
pub fn accent_dim() -> Color32 {
    cur().accent_dim
}

#[inline]
fn lerp_rgb(from: Color32, to: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let lerp = |a: u8, b: u8| -> u8 {
        (a as f32 + (b as f32 - a as f32) * t).round() as u8
    };
    Color32::from_rgb(lerp(from.r(), to.r()), lerp(from.g(), to.g()), lerp(from.b(), to.b()))
}

pub fn editor_zebra_alt_fill(editor_bg: Color32) -> Color32 {
    let c = cur();
    lerp_rgb(editor_bg, c.accent, 0.032)
}

pub fn editor_zebra_rel_line_num_on_stripe() -> Color32 {
    let c = cur();
    lerp_rgb(c.accent_dim, c.accent, 0.14)
}

pub fn start_green() -> Color32 {
    cur().accent
}
pub fn start_green_dim() -> Color32 {
    cur().accent_dim
}
pub fn focus() -> Color32 {
    cur().focus
}
pub fn literal_num() -> Color32 {
    cur().literal_num
}
pub fn label_cyan() -> Color32 {
    cur().label_cyan
}
pub fn err_red() -> Color32 {
    cur().err_red
}
pub fn dim_gray() -> Color32 {
    cur().dim_gray
}
pub fn section() -> Color32 {
    cur().section
}
pub fn panel_deep() -> Color32 {
    cur().panel_deep
}
pub fn panel_mid() -> Color32 {
    cur().panel_mid
}
pub fn panel_lift() -> Color32 {
    cur().panel_lift
}
pub fn button_fill_strong() -> Color32 {
    cur().button_fill_strong
}
pub fn disabled_panel() -> Color32 {
    cur().disabled_panel
}
pub fn editor_placeholder() -> Color32 {
    cur().editor_placeholder
}
pub fn search_bg() -> Color32 {
    cur().search_bg
}
pub fn match_dim() -> Color32 {
    cur().match_dim
}
pub fn match_cur() -> Color32 {
    cur().match_cur
}
pub fn sim_surface() -> Color32 {
    cur().sim_surface
}
pub fn sim_surface_lift() -> Color32 {
    cur().sim_surface_lift
}
pub fn sim_tab_active() -> Color32 {
    cur().sim_tab_active
}
pub fn sim_border() -> Color32 {
    cur().sim_border
}
pub fn sim_border_bright() -> Color32 {
    cur().sim_border_bright
}
pub fn sim_stop_fill() -> Color32 {
    cur().sim_stop_fill
}
pub fn sim_stop_border() -> Color32 {
    cur().sim_stop_border
}
pub fn main_central_fill() -> Color32 {
    cur().main_central_fill
}

const WALLPAPER_VISIBLE_ID_STR: &str = "fm_fullscreen_wallpaper";

pub fn set_wallpaper_visible(ctx: &egui::Context, visible: bool) {
    ctx.data_mut(|d| d.insert_temp(Id::new(WALLPAPER_VISIBLE_ID_STR), visible));
}

fn wallpaper_visible(ctx: &egui::Context) -> bool {
    ctx.data(|d| d.get_temp(Id::new(WALLPAPER_VISIBLE_ID_STR))).unwrap_or(false)
}

pub fn panel_over_wallpaper(ctx: &egui::Context, base: Color32) -> Color32 {
    if !wallpaper_visible(ctx) {
        return base;
    }
    let (r, g, b) = (base.r() as f32, base.g() as f32, base.b() as f32);
    let y = (0.2126 * r + 0.7152 * g + 0.0722 * b) / 255.0;
    let a = 92.0 + (1.0 - y) * 58.0;
    let a = a.clamp(82.0, 158.0) as u8;
    Color32::from_rgba_unmultiplied(base.r(), base.g(), base.b(), a)
}

pub fn text_primary() -> Color32 {
    cur().text_primary
}
pub fn on_accent_text() -> Color32 {
    cur().on_accent_text
}
pub fn status_error() -> Color32 {
    cur().status_error
}
pub fn err_red_soft() -> Color32 {
    cur().err_red_soft
}
pub fn syntax_punct() -> Color32 {
    cur().syntax_punct
}
pub fn syntax_ws() -> Color32 {
    cur().syntax_ws
}
pub fn periph_dim() -> Color32 {
    cur().periph_dim
}
pub fn periph_pin_used() -> Color32 {
    cur().periph_pin_used
}

pub fn widget_corner_radius() -> CornerRadius {
    match chrome() {
        ChromeProfile::Standard => CornerRadius::same(5),
        ChromeProfile::VsCodeStyle => CornerRadius::same(3),
    }
}

fn paint_combo_chevron(
    ui: &Ui,
    rect: egui::Rect,
    visuals: &egui::style::WidgetVisuals,
    is_open: bool,
    above_or_below: AboveOrBelow,
) {
    let t = ui
        .ctx()
        .animate_bool_responsive(ui.id().with("__fm_combo_chevron"), is_open);
    let dir = match above_or_below {
        AboveOrBelow::Below => 1.0,
        AboveOrBelow::Above => -1.0,
    };
    let rot = Rot2::from_angle(std::f32::consts::PI * t * dir);
    let center = rect.center();
    let w = rect.width() * 0.38;
    let h = rect.height() * 0.32;
    let p0 = pos2(-w, -h);
    let p1 = pos2(0.0, h);
    let p2 = pos2(w, -h);
    let xform = |p: Pos2| center + rot * p.to_vec2();
    let stroke = Stroke::new(1.35, visuals.fg_stroke.color);
    let painter = ui.painter();
    painter.line_segment([xform(p0), xform(p1)], stroke);
    painter.line_segment([xform(p1), xform(p2)], stroke);
}

pub fn combo_box(id_salt: impl std::hash::Hash) -> ComboBox {
    ComboBox::from_id_salt(id_salt).icon(paint_combo_chevron)
}

pub fn apply_dropdown_menu_style(ui: &mut Ui) {
    let p = cur();
    let fill = panel_over_wallpaper(ui.ctx(), p.panel_mid);
    let v = &mut ui.style_mut().visuals;
    let bottom_r = widget_corner_radius().nw;
    v.menu_corner_radius = CornerRadius {
        nw: 0,
        ne: 0,
        sw: bottom_r,
        se: bottom_r,
    };
    v.window_corner_radius = v.menu_corner_radius;
    v.window_fill = fill;
    v.window_stroke = Stroke::new(1.0, p.accent_dim);
    v.override_text_color = Some(p.text_primary);
    // Drop shadow reads as a detached layer above the menubar; keep the shell flat.
    v.popup_shadow = Shadow::NONE;
    let row_r = CornerRadius::same(6);
    v.widgets.noninteractive.corner_radius = row_r;
    v.widgets.inactive.corner_radius = row_r;
    v.widgets.hovered.corner_radius = row_r;
    v.widgets.active.corner_radius = row_r;
    v.widgets.open.corner_radius = row_r;
}

pub fn apply_egui_visuals(ctx: &egui::Context) {
    let p = cur();
    let a = p.accent;
    let a_dim = p.accent_dim;
    let wround = widget_corner_radius();
    let vscode_chr = matches!(chrome(), ChromeProfile::VsCodeStyle);
    let mut visuals = Visuals::dark();
    visuals.override_text_color = Some(p.text_primary);
    visuals.extreme_bg_color = p.main_central_fill;
    visuals.faint_bg_color = p.main_central_fill;
    visuals.panel_fill = p.main_central_fill;
    visuals.window_fill = p.panel_lift;
    visuals.code_bg_color = p.main_central_fill;
    visuals.menu_corner_radius = wround;
    visuals.window_corner_radius = wround;
    let border_a = if vscode_chr { 130 } else { 200 };
    visuals.window_stroke = Stroke::new(
        if vscode_chr { 0.85 } else { 1.0 },
        Color32::from_rgba_unmultiplied(a_dim.r(), a_dim.g(), a_dim.b(), border_a),
    );
    visuals.popup_shadow = if vscode_chr {
        Shadow {
            offset: [0, 3],
            blur: 10,
            spread: 0,
            color: Color32::from_rgba_unmultiplied(0, 0, 0, 48),
        }
    } else {
        Shadow {
            offset: [4, 12],
            blur: 20,
            spread: 0,
            color: Color32::from_rgba_unmultiplied(0, 0, 0, 100),
        }
    };

    let black_widget = |w: &mut egui::style::WidgetVisuals| {
        w.bg_fill = p.main_central_fill;
        w.bg_stroke = Stroke::NONE;
    };
    black_widget(&mut visuals.widgets.noninteractive);
    visuals.widgets.noninteractive.weak_bg_fill = p.panel_deep;
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, p.accent_dim);
    visuals.widgets.noninteractive.corner_radius = wround;

    visuals.widgets.inactive.bg_fill = Color32::from_rgba_unmultiplied(
        p.panel_deep.r(),
        p.panel_deep.g(),
        p.panel_deep.b(),
        200,
    );
    visuals.widgets.inactive.weak_bg_fill = p.panel_lift;
    visuals.widgets.inactive.bg_stroke =
        Stroke::new(0.85, Color32::from_rgba_unmultiplied(a_dim.r(), a_dim.g(), a_dim.b(), 55));
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, p.text_primary);
    visuals.widgets.inactive.corner_radius = wround;

    visuals.widgets.hovered.bg_fill = Color32::from_rgba_unmultiplied(a.r(), a.g(), a.b(), 18);
    visuals.widgets.hovered.weak_bg_fill = Color32::from_rgba_unmultiplied(a.r(), a.g(), a.b(), 32);
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, a_dim);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, p.accent);
    visuals.widgets.hovered.corner_radius = wround;
    visuals.widgets.hovered.expansion = 0.2;

    visuals.widgets.active.bg_fill = Color32::from_rgba_unmultiplied(a.r(), a.g(), a.b(), 42);
    visuals.widgets.active.weak_bg_fill = Color32::from_rgba_unmultiplied(a.r(), a.g(), a.b(), 52);
    visuals.widgets.active.bg_stroke = Stroke::new(1.0, a);
    visuals.widgets.active.fg_stroke = Stroke::new(1.1, p.focus);
    visuals.widgets.active.corner_radius = wround;
    visuals.widgets.active.expansion = 0.2;

    visuals.widgets.open.bg_fill = Color32::from_rgba_unmultiplied(a.r(), a.g(), a.b(), 40);
    visuals.widgets.open.weak_bg_fill = Color32::from_rgba_unmultiplied(a.r(), a.g(), a.b(), 46);
    visuals.widgets.open.bg_stroke = Stroke::new(1.0, a);
    visuals.widgets.open.fg_stroke = Stroke::new(1.0, p.focus);
    visuals.widgets.open.corner_radius = wround;
    visuals.widgets.open.expansion = 0.2;

    visuals.text_cursor.stroke = Stroke::new(2.0, p.text_primary);
    visuals.selection.bg_fill = p.selection_bg;
    visuals.selection.stroke = Stroke::new(1.0, p.text_primary);
    visuals.slider_trailing_fill = true;
    visuals.handle_shape = HandleShape::Circle;
    ctx.set_visuals(visuals);
    ctx.style_mut(|s| {
        s.spacing.menu_spacing = 0.0;
        if vscode_chr {
            s.spacing.slider_rail_height = 4.0;
            s.spacing.item_spacing = egui::vec2(8.0, 4.0);
            s.spacing.button_padding = egui::vec2(8.0, 4.0);
            s.spacing.interact_size = egui::vec2(16.0, 24.0);
            s.text_styles
                .insert(TextStyle::Small, FontId::new(12.0, FontFamily::Proportional));
            s.text_styles
                .insert(TextStyle::Body, FontId::new(13.0, FontFamily::Proportional));
            s.text_styles.insert(
                TextStyle::Monospace,
                FontId::new(13.5, FontFamily::Monospace),
            );
            s.text_styles
                .insert(TextStyle::Button, FontId::new(13.0, FontFamily::Proportional));
            s.text_styles
                .insert(TextStyle::Heading, FontId::new(17.0, FontFamily::Proportional));
        } else {
            s.spacing.slider_rail_height = 6.0;
            s.spacing.item_spacing = egui::vec2(8.0, 4.0);
            s.spacing.button_padding = egui::vec2(8.0, 4.0);
            s.spacing.interact_size = egui::vec2(16.0, 24.0);
            s.text_styles
                .insert(TextStyle::Small, FontId::new(11.5, FontFamily::Proportional));
            s.text_styles
                .insert(TextStyle::Body, FontId::new(13.5, FontFamily::Proportional));
            s.text_styles.insert(
                TextStyle::Monospace,
                FontId::new(14.0, FontFamily::Monospace),
            );
            s.text_styles
                .insert(TextStyle::Button, FontId::new(13.0, FontFamily::Proportional));
            s.text_styles
                .insert(TextStyle::Heading, FontId::new(18.0, FontFamily::Proportional));
        }
    });
}

/// Labels for the customization panel (order matches [`theme_palette_field_mut`]).
pub const PALETTE_SLIDER_LABELS: &[&str] = &[
    "Accent",
    "Accent dim",
    "Focus",
    "Literal / number",
    "Label cyan",
    "Error red",
    "Dim gray",
    "Section",
    "Panel deep",
    "Panel mid",
    "Panel lift",
    "Button fill strong",
    "Disabled panel",
    "Editor placeholder",
    "Search background",
    "Match (dim)",
    "Match (current)",
    "Sim surface",
    "Sim surface lift",
    "Sim tab active",
    "Sim border",
    "Sim border bright",
    "Sim stop fill",
    "Sim stop border",
    "Main / central background",
    "Text",
    "Text on accent",
    "Status error",
    "Error red (soft)",
    "Syntax punct",
    "Syntax ws",
    "Peripheral dim",
    "Peripheral pin (used / high)",
    "Selection",
];

/// Mutable access for one palette field by index (0..`ThemePalette::FIELD_COUNT`).
pub fn theme_palette_field_mut(
    p: &mut ThemePalette,
    i: usize,
) -> Option<(&'static str, &mut Color32)> {
    let label: &'static str = PALETTE_SLIDER_LABELS.get(i).copied()?;
    let c: &mut Color32 = match i {
        0 => &mut p.accent,
        1 => &mut p.accent_dim,
        2 => &mut p.focus,
        3 => &mut p.literal_num,
        4 => &mut p.label_cyan,
        5 => &mut p.err_red,
        6 => &mut p.dim_gray,
        7 => &mut p.section,
        8 => &mut p.panel_deep,
        9 => &mut p.panel_mid,
        10 => &mut p.panel_lift,
        11 => &mut p.button_fill_strong,
        12 => &mut p.disabled_panel,
        13 => &mut p.editor_placeholder,
        14 => &mut p.search_bg,
        15 => &mut p.match_dim,
        16 => &mut p.match_cur,
        17 => &mut p.sim_surface,
        18 => &mut p.sim_surface_lift,
        19 => &mut p.sim_tab_active,
        20 => &mut p.sim_border,
        21 => &mut p.sim_border_bright,
        22 => &mut p.sim_stop_fill,
        23 => &mut p.sim_stop_border,
        24 => &mut p.main_central_fill,
        25 => &mut p.text_primary,
        26 => &mut p.on_accent_text,
        27 => &mut p.status_error,
        28 => &mut p.err_red_soft,
        29 => &mut p.syntax_punct,
        30 => &mut p.syntax_ws,
        31 => &mut p.periph_dim,
        32 => &mut p.periph_pin_used,
        33 => &mut p.selection_bg,
        _ => return None,
    };
    Some((label, c))
}

impl ThemePalette {
    pub const FIELD_COUNT: usize = 34;
}

pub fn modifiable_premade_presets() -> BTreeMap<String, ThemePalette> {
    [
        ("Green".to_string(), premade_green()),
        ("Red".to_string(), premade_red()),
        ("Blue".to_string(), premade_blue()),
        ("Matrix".to_string(), premade_matrix()),
        ("ACES".to_string(), premade_aces()),
    ]
    .into_iter()
    .collect()
}

fn premade_green() -> ThemePalette {
    let h = (60u8, 200u8, 120u8);
    let h_dim = (36, 120, 78);
    ThemePalette {
        accent:            Color32::from_rgb(h.0, h.1, h.2),
        accent_dim:        Color32::from_rgb(h_dim.0, h_dim.1, h_dim.2),
        focus:             Color32::from_rgb(110, 230, 170),
        literal_num:       Color32::from_rgb(150, 230, 200),
        label_cyan:        Color32::from_rgb(100, 220, 195),
        err_red:           Color32::from_rgb(255, 100, 100),
        dim_gray:          Color32::from_rgb(72, 90, 82),
        section:           Color32::from_rgb(120, 200, 160),
        panel_deep:        Color32::from_rgb(2, 8, 5),
        panel_mid:         Color32::from_rgb(4, 12, 8),
        panel_lift:        Color32::from_rgb(8, 18, 12),
        button_fill_strong: Color32::from_rgb(10, 24, 16),
        disabled_panel:   Color32::from_rgb(20, 28, 24),
        editor_placeholder: Color32::from_rgb(52, 75, 62),
        search_bg:         Color32::from_rgb(2, 6, 4),
        match_dim:         Color32::from_rgb(50, 100, 80),
        match_cur:         Color32::from_rgb(160, 255, 210),
        sim_surface:       Color32::from_rgb(4, 10, 7),
        sim_surface_lift:  Color32::from_rgb(8, 16, 11),
        sim_tab_active:    Color32::from_rgb(6, 14, 10),
        sim_border:        Color32::from_rgb(48, 88, 68),
        sim_border_bright: Color32::from_rgb(100, 180, 130),
        sim_stop_fill:     Color32::from_rgb(18, 10, 12),
        sim_stop_border:   Color32::from_rgb(120, 80, 90),
        main_central_fill: Color32::from_rgb(0, 0, 0),
        text_primary:      Color32::from_rgb(235, 245, 240),
        on_accent_text:    Color32::from_rgb(6, 18, 10),
        status_error:      Color32::from_rgb(255, 140, 130),
        err_red_soft:      Color32::from_rgb(200, 120, 120),
        syntax_punct:      Color32::from_rgb(85, 95, 90),
        syntax_ws:         Color32::from_rgb(48, 55, 50),
        periph_dim:        Color32::from_rgb(100, 130, 90),
        periph_pin_used:   Color32::from_rgb(90, 255, 150),
        selection_bg:      Color32::from_rgb(28, 60, 44),
    }
}

fn premade_red() -> ThemePalette {
    let a  = (240u8, 72u8, 64u8);
    let d  = (130, 55, 52);
    ThemePalette {
        accent:            Color32::from_rgb(a.0, a.1, a.2),
        accent_dim:        Color32::from_rgb(d.0, d.1, d.2),
        focus:             Color32::from_rgb(255, 140, 128),
        literal_num:       Color32::from_rgb(255, 160, 150),
        label_cyan:        Color32::from_rgb(255, 130, 120),
        err_red:           Color32::from_rgb(255, 80, 80),
        dim_gray:          Color32::from_rgb(85, 68, 68),
        section:           Color32::from_rgb(220, 120, 115),
        panel_deep:        Color32::from_rgb(10, 4, 4),
        panel_mid:         Color32::from_rgb(16, 7, 7),
        panel_lift:        Color32::from_rgb(22, 10, 10),
        button_fill_strong: Color32::from_rgb(28, 12, 12),
        disabled_panel:   Color32::from_rgb(36, 24, 24),
        editor_placeholder: Color32::from_rgb(95, 70, 70),
        search_bg:         Color32::from_rgb(8, 3, 3),
        match_dim:         Color32::from_rgb(120, 60, 58),
        match_cur:         Color32::from_rgb(255, 190, 185),
        sim_surface:       Color32::from_rgb(12, 5, 5),
        sim_surface_lift:  Color32::from_rgb(18, 8, 8),
        sim_tab_active:    Color32::from_rgb(14, 6, 6),
        sim_border:        Color32::from_rgb(95, 55, 55),
        sim_border_bright: Color32::from_rgb(200, 120, 115),
        sim_stop_fill:     Color32::from_rgb(20, 8, 10),
        sim_stop_border:   Color32::from_rgb(150, 70, 80),
        main_central_fill: Color32::from_rgb(0, 0, 0),
        text_primary:      Color32::from_rgb(255, 250, 248),
        on_accent_text:    Color32::from_rgb(18, 4, 4),
        status_error:      Color32::from_rgb(255, 120, 100),
        err_red_soft:      Color32::from_rgb(210, 100, 100),
        syntax_punct:      Color32::from_rgb(95, 80, 80),
        syntax_ws:         Color32::from_rgb(58, 48, 48),
        periph_dim:        Color32::from_rgb(140, 100, 45),
        periph_pin_used:   Color32::from_rgb(255, 190, 130),
        selection_bg:      Color32::from_rgb(70, 30, 28),
    }
}

fn premade_blue() -> ThemePalette {
    ThemePalette {
        accent:            Color32::from_rgb(66, 160, 255),
        accent_dim:        Color32::from_rgb(55, 100, 150),
        focus:             Color32::from_rgb(140, 200, 255),
        literal_num:       Color32::from_rgb(170, 210, 255),
        label_cyan:        Color32::from_rgb(100, 200, 255),
        err_red:           Color32::from_rgb(255, 100, 100),
        dim_gray:          Color32::from_rgb(70, 78, 92),
        section:           Color32::from_rgb(130, 175, 230),
        panel_deep:        Color32::from_rgb(3, 6, 14),
        panel_mid:         Color32::from_rgb(5, 10, 20),
        panel_lift:        Color32::from_rgb(10, 16, 28),
        button_fill_strong: Color32::from_rgb(12, 20, 36),
        disabled_panel:   Color32::from_rgb(24, 28, 38),
        editor_placeholder: Color32::from_rgb(65, 80, 105),
        search_bg:         Color32::from_rgb(3, 5, 12),
        match_dim:         Color32::from_rgb(70, 110, 160),
        match_cur:         Color32::from_rgb(200, 225, 255),
        sim_surface:       Color32::from_rgb(5, 8, 18),
        sim_surface_lift:  Color32::from_rgb(10, 14, 26),
        sim_tab_active:    Color32::from_rgb(8, 12, 24),
        sim_border:        Color32::from_rgb(60, 85, 120),
        sim_border_bright: Color32::from_rgb(140, 175, 220),
        sim_stop_fill:     Color32::from_rgb(15, 10, 18),
        sim_stop_border:   Color32::from_rgb(110, 80, 110),
        main_central_fill: Color32::from_rgb(0, 0, 0),
        text_primary:      Color32::from_rgb(248, 250, 255),
        on_accent_text:    Color32::from_rgb(4, 8, 18),
        status_error:      Color32::from_rgb(255, 140, 130),
        err_red_soft:      Color32::from_rgb(200, 120, 120),
        syntax_punct:      Color32::from_rgb(88, 95, 108),
        syntax_ws:         Color32::from_rgb(50, 55, 65),
        periph_dim:        Color32::from_rgb(110, 95, 50),
        periph_pin_used:   Color32::from_rgb(120, 200, 255),
        selection_bg:      Color32::from_rgb(35, 55, 85),
    }
}

fn premade_matrix() -> ThemePalette {
    ThemePalette {
        accent:            Color32::from_rgb(0, 255, 65),
        accent_dim:        Color32::from_rgb(0, 160, 45),
        focus:             Color32::from_rgb(80, 255, 120),
        literal_num:       Color32::from_rgb(0, 255, 80),
        label_cyan:        Color32::from_rgb(0, 220, 90),
        err_red:           Color32::from_rgb(255, 60, 60),
        dim_gray:          Color32::from_rgb(0, 90, 35),
        section:           Color32::from_rgb(0, 200, 70),
        panel_deep:        Color32::from_rgb(0, 6, 2),
        panel_mid:         Color32::from_rgb(0, 12, 4),
        panel_lift:        Color32::from_rgb(0, 18, 6),
        button_fill_strong: Color32::from_rgb(0, 22, 8),
        disabled_panel:   Color32::from_rgb(0, 30, 12),
        editor_placeholder: Color32::from_rgb(0, 80, 30),
        search_bg:         Color32::from_rgb(0, 0, 0),
        match_dim:         Color32::from_rgb(0, 100, 40),
        match_cur:         Color32::from_rgb(0, 255, 100),
        sim_surface:       Color32::from_rgb(0, 8, 3),
        sim_surface_lift:  Color32::from_rgb(0, 14, 5),
        sim_tab_active:    Color32::from_rgb(0, 10, 4),
        sim_border:        Color32::from_rgb(0, 100, 40),
        sim_border_bright: Color32::from_rgb(0, 200, 80),
        sim_stop_fill:     Color32::from_rgb(12, 4, 6),
        sim_stop_border:   Color32::from_rgb(100, 40, 50),
        main_central_fill: Color32::from_rgb(0, 0, 0),
        text_primary:      Color32::from_rgb(0, 255, 70),
        on_accent_text:    Color32::from_rgb(0, 0, 0),
        status_error:      Color32::from_rgb(255, 80, 80),
        err_red_soft:      Color32::from_rgb(200, 90, 90),
        syntax_punct:      Color32::from_rgb(0, 120, 45),
        syntax_ws:         Color32::from_rgb(0, 45, 18),
        periph_dim:        Color32::from_rgb(0, 100, 40),
        periph_pin_used:   Color32::from_rgb(0, 255, 90),
        selection_bg:      Color32::from_rgb(0, 55, 20),
    }
}

/// this WILL be improved, aces doenst deserve this
fn premade_aces() -> ThemePalette {
    let gold = (255u8, 204u8, 0u8);
    let gold_d = (180u8, 145u8, 0u8);
    ThemePalette {
        accent:            Color32::from_rgb(gold.0, gold.1, gold.2),
        accent_dim:        Color32::from_rgb(gold_d.0, gold_d.1, gold_d.2),
        focus:             Color32::from_rgb(255, 230, 120),
        literal_num:       Color32::from_rgb(255, 220, 100),
        label_cyan:        Color32::from_rgb(255, 200, 80),
        err_red:           Color32::from_rgb(255, 90, 90),
        dim_gray:          Color32::from_rgb(140, 130, 110),
        section:           Color32::from_rgb(240, 200, 60),
        panel_deep:        Color32::from_rgb(0, 0, 0),
        panel_mid:         Color32::from_rgb(12, 10, 4),
        panel_lift:        Color32::from_rgb(20, 16, 6),
        button_fill_strong: Color32::from_rgb(22, 18, 6),
        disabled_panel:   Color32::from_rgb(32, 28, 16),
        editor_placeholder: Color32::from_rgb(85, 78, 55),
        search_bg:         Color32::from_rgb(0, 0, 0),
        match_dim:         Color32::from_rgb(120, 100, 40),
        match_cur:         Color32::from_rgb(255, 240, 160),
        sim_surface:       Color32::from_rgb(4, 4, 0),
        sim_surface_lift:  Color32::from_rgb(10, 8, 2),
        sim_tab_active:    Color32::from_rgb(8, 6, 0),
        sim_border:        Color32::from_rgb(120, 100, 40),
        sim_border_bright: Color32::from_rgb(230, 200, 60),
        sim_stop_fill:     Color32::from_rgb(18, 6, 6),
        sim_stop_border:   Color32::from_rgb(130, 60, 60),
        main_central_fill: Color32::from_rgb(0, 0, 0),
        text_primary:      Color32::from_rgb(255, 255, 255),
        on_accent_text:    Color32::from_rgb(0, 0, 0),
        status_error:      Color32::from_rgb(255, 120, 100),
        err_red_soft:      Color32::from_rgb(200, 100, 95),
        syntax_punct:      Color32::from_rgb(160, 150, 120),
        syntax_ws:         Color32::from_rgb(60, 55, 45),
        periph_dim:        Color32::from_rgb(180, 150, 40),
        periph_pin_used:   Color32::from_rgb(255, 220, 50),
        selection_bg:      Color32::from_rgb(70, 55, 10),
    }
}
