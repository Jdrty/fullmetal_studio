//! customization, not the best at the moment, limited user direction

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use eframe::egui::{
    self, pos2, vec2, Align, Align2, Button, Color32, CornerRadius, Frame, Id, Label,
    Layout, Margin, Order, Rect, RichText, ScrollArea, Sense, Slider, Stroke, StrokeKind, TextEdit,
    TextWrapMode, Window,
};

use eframe::egui::widgets::color_picker::{show_color_at, Alpha};

use crate::clipped_color_picker::color_picker_color32;

use crate::theme::{
    self, start_green, start_green_dim, theme_palette_field_mut, ThemePalette, ThemePaletteFile,
    PALETTE_SLIDER_LABELS,
};

pub const APP_SUPPORT_DIR_NAME: &str = "Full Metal Studio";
pub const ONE_DARK_PRO_ACTIVE: &str = "one_dark_pro";
const PRESET_FILE: &str = "color_presets.json";
const LAST_WORKSPACE_FILE: &str = "last_workspace.json";
const JSON_VERSION: u32 = 2;

/// editor must never be fully covered NEVERR
pub const MAX_WALLPAPER_ALPHA: f32 = 0.85;
pub const MIN_WALLPAPER_ALPHA: f32 = 0.02;

/// editor wallpaper stored in `color_presets.json` (depending on whether or not I put effort into
/// wallpaper implimentation, this will have its own file)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct WallpaperSettings {      // shitty attempt to make wallpapers usable
    pub enabled: bool,
    pub path:      String,
    #[serde(default = "default_wallpaper_alpha")]
    pub alpha:     f32,
    #[serde(default)]
    pub blur:      f32,
    #[serde(default)]
    pub corner_smooth: f32,
}

fn default_wallpaper_alpha() -> f32 {
    0.28
}

impl Default for WallpaperSettings {
    fn default() -> Self {
        Self {
            enabled:      false,
            path:         String::new(),
            alpha:        default_wallpaper_alpha(),
            blur:         0.0,
            corner_smooth: 0.0,
        }
    }
}

impl WallpaperSettings {
    pub fn clamp_alpha(&mut self) {
        self.alpha = self
            .alpha
            .clamp(MIN_WALLPAPER_ALPHA, MAX_WALLPAPER_ALPHA);
        self.blur         = self.blur.clamp(0.0, 1.0);
        self.corner_smooth = self.corner_smooth.clamp(0.0, 1.0);
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct SavedPresetBody {
    #[serde(flatten)]
    palette:   ThemePaletteFile,
    #[serde(default)]
    wallpaper: WallpaperSettings,
}

/// on-disk v2
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct PresetFileV2 {
    version: u32,
    active:  String,
    #[serde(default)]
    default_wallpaper: WallpaperSettings,
    #[serde(default)]
    one_dark_pro_wallpaper: WallpaperSettings,
    #[serde(default)]
    vscode_style_chrome: bool,
    #[serde(default)]
    presets:   BTreeMap<String, SavedPresetBody>,
}

/// legacy on-disk (colors only)
#[derive(Clone, Debug, serde::Deserialize)]
struct PresetFileV1 {
    version: u32,
    active:  String,
    #[serde(default)]
    presets:   BTreeMap<String, ThemePaletteFile>,
}

/// data loaded from `color_presets.json`
pub struct LoadedPresets {
    pub active:                 String,
    pub user_presets:           BTreeMap<String, ThemePalette>,
    pub default_wallpaper:      WallpaperSettings,
    pub wallpaper_for_named:    BTreeMap<String, WallpaperSettings>,
    pub vscode_style_chrome:    bool,
}

#[derive(Debug, Clone)]
pub struct SessionSnapshot {
    pub color_palette:         ThemePalette,
    pub color_active:          String,
    pub user_presets:          BTreeMap<String, ThemePalette>,
    pub default_wallpaper:     WallpaperSettings,
    pub wallpaper_for_named:   BTreeMap<String, WallpaperSettings>,
    pub vscode_style_chrome:   bool,
}

/// macOS: `~/Library/Application Support/Full Metal Studio/`
/// other: `~/.config/Full Metal Studio/` 
/// im gonna be honest, I have ZERO confidence that this works on anything but a mac
fn app_support_dir() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    let mut p = PathBuf::from(home);
    if cfg!(target_os = "macos") {
        p.push("Library");
        p.push("Application Support");
    } else {
        p.push(".config");
    }
    p.push(APP_SUPPORT_DIR_NAME);
    Some(p)
}

fn preset_path() -> Option<PathBuf> {
    let mut p = app_support_dir()?;
    p.push(PRESET_FILE);
    Some(p)
}

fn last_workspace_path() -> Option<PathBuf> {
    let mut p = app_support_dir()?;
    p.push(LAST_WORKSPACE_FILE);
    Some(p)
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct LastWorkspaceFile {
    version: u32,
    root: String,
}

pub fn load_last_workspace_dir() -> Option<PathBuf> {
    let path = last_workspace_path()?;
    let data = fs::read(&path).ok()?;
    let f: LastWorkspaceFile = serde_json::from_slice(&data).ok()?;
    if f.version != 1 || f.root.is_empty() {
        return None;
    }
    Some(PathBuf::from(f.root))
}

pub fn save_last_workspace_dir(root: &Path, scratch_root: &Path) -> Result<(), String> {
    if root == scratch_root {
        return Ok(());
    }
    let Some(dir) = app_support_dir() else {
        return Err("Could not resolve app data directory.".to_string());
    };
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let f = LastWorkspaceFile {
        version: 1,
        root: root.to_string_lossy().to_string(),
    };
    let json = serde_json::to_string_pretty(&f).map_err(|e| e.to_string())?;
    fs::write(dir.join(LAST_WORKSPACE_FILE), json).map_err(|e| e.to_string())
}

fn strip_reserved_preset_keys(presets: &mut BTreeMap<String, ThemePalette>) {
    presets.remove(ONE_DARK_PRO_ACTIVE);
}

/// which palette the app should use for `active` / stored names
pub fn active_palette(active: &str, presets: &BTreeMap<String, ThemePalette>) -> ThemePalette {
    match active {
        "default" => ThemePalette::DEFAULT,
        ONE_DARK_PRO_ACTIVE => ThemePalette::ONE_DARK_PRO,
        _ => presets
            .get(active)
            .copied()
            .unwrap_or(ThemePalette::DEFAULT),
    }
}

/// first launch or missing/invalid `color_presets.json`: One Dark Pro colors and VS Code chrome
/// not a fan of it but tbf it looks better than the one I like (to most ppl atleast)
/// whatever i might change it again
fn default_first_launch_presets() -> LoadedPresets {
    LoadedPresets {
        active:              ONE_DARK_PRO_ACTIVE.to_string(),
        user_presets:        BTreeMap::new(),
        default_wallpaper:   WallpaperSettings::default(),
        wallpaper_for_named: BTreeMap::new(),
        vscode_style_chrome: true,
    }
}

pub fn load_stored() -> LoadedPresets {
    let Some(path) = preset_path() else {
        return default_first_launch_presets();
    };
    let data = match fs::read(&path) {
        Ok(b) => b,
        Err(_) => {
            return default_first_launch_presets();
        }
    };
    if let Ok(f) = serde_json::from_slice::<PresetFileV2>(&data) {
        if f.version > JSON_VERSION {
            return default_first_launch_presets();
        }
        let mut user_presets          = BTreeMap::new();
        let mut wallpaper_for_named   = BTreeMap::new();
        let mut odp_wallpaper_migrated = None::<WallpaperSettings>;
        for (k, body) in f.presets {
            if k == ONE_DARK_PRO_ACTIVE {
                odp_wallpaper_migrated = Some(body.wallpaper);
                continue;
            }
            user_presets.insert(k.clone(), ThemePalette::from(body.palette));
            wallpaper_for_named.insert(k, body.wallpaper);
        }
        strip_reserved_preset_keys(&mut user_presets);
        let mut odp_w = f.one_dark_pro_wallpaper;
        odp_w.clamp_alpha();
        let odp_empty = !odp_w.enabled && odp_w.path.is_empty();
        if odp_empty {
            if let Some(mut w) = odp_wallpaper_migrated {
                w.clamp_alpha();
                odp_w = w;
            }
        }
        wallpaper_for_named.insert(ONE_DARK_PRO_ACTIVE.to_string(), odp_w);
        let active = if f.active == "default" {
            "default".to_string()
        } else if f.active == ONE_DARK_PRO_ACTIVE {
            ONE_DARK_PRO_ACTIVE.to_string()
        } else if user_presets.contains_key(&f.active) {
            f.active
        } else {
            "default".to_string()
        };
        let mut def = f.default_wallpaper;
        def.clamp_alpha();
        for w in wallpaper_for_named.values_mut() {
            w.clamp_alpha();
        }
        return LoadedPresets {
            active,
            user_presets,
            default_wallpaper: def,
            wallpaper_for_named,
            vscode_style_chrome: f.vscode_style_chrome,
        };
    }
    if let Ok(v1) = serde_json::from_slice::<PresetFileV1>(&data) {
        if v1.version <= 1 {
            let map: BTreeMap<String, ThemePalette> = v1
                .presets
                .into_iter()
                .filter(|(k, _)| k != ONE_DARK_PRO_ACTIVE)
                .map(|(k, v)| (k, ThemePalette::from(v)))
                .collect();
            let mut map = map;
            strip_reserved_preset_keys(&mut map);
            let active = if v1.active == "default" {
                "default".to_string()
            } else if v1.active == ONE_DARK_PRO_ACTIVE {
                ONE_DARK_PRO_ACTIVE.to_string()
            } else if map.contains_key(&v1.active) {
                v1.active
            } else {
                "default".to_string()
            };
            return LoadedPresets {
                active,
                user_presets: map,
                default_wallpaper: WallpaperSettings::default(),
                wallpaper_for_named: BTreeMap::new(),
                vscode_style_chrome: false,
            };
        }
    }
    default_first_launch_presets()
}

pub fn save_to_disk(
    active:              &str,
    presets:             &BTreeMap<String, ThemePalette>,
    default_wallpaper:   &WallpaperSettings,
    wallpaper_for_named: &BTreeMap<String, WallpaperSettings>,
    vscode_style_chrome: bool,
) -> Result<(), String> {
    let Some(dir) = app_support_dir() else {
        return Err("Could not resolve app data directory.".to_string());
    };
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let mut def = default_wallpaper.clone();
    def.clamp_alpha();
    let mut presets_json = BTreeMap::new();
    for (k, pal) in presets {
        if k == ONE_DARK_PRO_ACTIVE {
            continue;
        }
        let mut w = wallpaper_for_named.get(k).cloned().unwrap_or_default();
        w.clamp_alpha();
        presets_json.insert(
            k.clone(),
            SavedPresetBody {
                palette:   ThemePaletteFile::from(*pal),
                wallpaper: w,
            },
        );
    }
    let mut odp_wp = wallpaper_for_named
        .get(ONE_DARK_PRO_ACTIVE)
        .cloned()
        .unwrap_or_default();
    odp_wp.clamp_alpha();
    let f = PresetFileV2 {
        version: JSON_VERSION,
        active:  active.to_string(),
        default_wallpaper: def,
        one_dark_pro_wallpaper: odp_wp,
        vscode_style_chrome,
        presets: presets_json,
    };
    let json = serde_json::to_string_pretty(&f).map_err(|e| e.to_string())?;
    let path = dir.join(PRESET_FILE);
    fs::write(&path, json).map_err(|e| e.to_string())?;
    Ok(())
}

/// Wallpaper for the active preset (Default uses `default_wallpaper` in the file).
pub fn active_wallpaper(
    active:            &str,
    default_wallpaper: &WallpaperSettings,
    named:             &BTreeMap<String, WallpaperSettings>,
) -> WallpaperSettings {
    if active == "default" {
        let mut w = default_wallpaper.clone();
        w.clamp_alpha();
        w
    } else {
        let mut w = named.get(active).cloned().unwrap_or_default();
        w.clamp_alpha();
        w
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PresetChoice {
    Default,
    OneDarkPro,
    Custom(String),
}

pub struct AfterCustomizationApply {
    pub user_presets:        BTreeMap<String, ThemePalette>,
    pub choice:              PresetChoice,
    pub palette:             ThemePalette,
    pub default_wallpaper:   WallpaperSettings,
    pub wallpaper_for_named: BTreeMap<String, WallpaperSettings>,
    pub vscode_style_chrome: bool,
}

#[derive(Debug)]
pub struct CustomizationState {
    pub open:            bool,
    pub editing:         ThemePalette,
    pub undo_snapshot:   ThemePalette,
    pub selected:        PresetChoice,
    pub user_presets:    BTreeMap<String, ThemePalette>,
    pub editing_is_default: bool,
    pub default_wallpaper:  WallpaperSettings,
    pub wallpaper_by_name: BTreeMap<String, WallpaperSettings>,
    pub editing_wallpaper: WallpaperSettings,
    pub undo_wallpaper:    WallpaperSettings,
    pub vscode_style_chrome: bool,
    pub undo_vscode_chrome:  bool,
    pub add_name_buffer:    String,
    pub add_error:          Option<String>,
    pub name_prompt_open:        bool,
    name_prompt_request_focus:   bool,
    session_baseline:            Option<SessionSnapshot>,
    open_color_i:                Option<usize>,
    last_custom_window_rect:     Option<Rect>,
    side_color_dock_bump:        u32,
    custom_open_generation:      u32,
}

impl Default for CustomizationState {
    fn default() -> Self {
        Self {
            open:               false,
            editing:            ThemePalette::DEFAULT,
            undo_snapshot:      ThemePalette::DEFAULT,
            selected:           PresetChoice::Default,
            user_presets:       BTreeMap::new(),
            editing_is_default: true,
            default_wallpaper:  WallpaperSettings::default(),
            wallpaper_by_name:  BTreeMap::new(),
            editing_wallpaper:  WallpaperSettings::default(),
            undo_wallpaper:     WallpaperSettings::default(),
            vscode_style_chrome: false,
            undo_vscode_chrome:  false,
            add_name_buffer:    String::new(),
            add_error:          None,
            name_prompt_open:        false,
            name_prompt_request_focus: false,
            session_baseline:         None,
            open_color_i:            None,
            last_custom_window_rect: None,
            side_color_dock_bump:    0,
            custom_open_generation:  0,
        }
    }
}

impl CustomizationState {
    pub fn open_from_current(
        &mut self,
        applied:              &ThemePalette,
        active:               &str,
        user:                 &BTreeMap<String, ThemePalette>,
        default_wallpaper:    &WallpaperSettings,
        wallpaper_for_named:  &BTreeMap<String, WallpaperSettings>,
        vscode_style_chrome:  bool,
    ) {
        self.open              = true;
        self.custom_open_generation = self.custom_open_generation.wrapping_add(1);
        self.side_color_dock_bump   = 0;
        self.name_prompt_open        = false;
        self.name_prompt_request_focus = false;
        self.undo_snapshot = *applied;
        self.user_presets  = user.clone();
        self.default_wallpaper = default_wallpaper.clone();
        self.wallpaper_by_name = wallpaper_for_named.clone();
        if active == "default" {
            self.selected         = PresetChoice::Default;
            self.editing          = ThemePalette::DEFAULT;
            self.editing_is_default = true;
        } else if active == ONE_DARK_PRO_ACTIVE {
            self.selected           = PresetChoice::OneDarkPro;
            self.editing           = ThemePalette::ONE_DARK_PRO;
            self.editing_is_default = false;
        } else if let Some(p) = user.get(active) {
            self.selected         = PresetChoice::Custom(active.to_string());
            self.editing          = *p;
            self.editing_is_default = false;
        } else {
            self.selected         = PresetChoice::Default;
            self.editing          = ThemePalette::DEFAULT;
            self.editing_is_default = true;
        }
        self.editing_wallpaper = active_wallpaper(active, default_wallpaper, wallpaper_for_named);
        self.undo_wallpaper  = self.editing_wallpaper.clone();
        self.vscode_style_chrome = vscode_style_chrome;
        self.undo_vscode_chrome  = vscode_style_chrome;
        self.session_baseline = Some(SessionSnapshot {
            color_palette:       *applied,
            color_active:        active.to_string(),
            user_presets:        user.clone(),
            default_wallpaper:     default_wallpaper.clone(),
            wallpaper_for_named:   wallpaper_for_named.clone(),
            vscode_style_chrome,
        });
        self.open_color_i            = None;
        self.last_custom_window_rect = None;
    }

    fn selected_label(&self) -> String {
        match &self.selected {
            PresetChoice::Default     => "Default".to_string(),
            PresetChoice::OneDarkPro  => "One Dark Pro".to_string(),
            PresetChoice::Custom(n)   => n.clone(),
        }
    }

    fn apply_add_named_preset(&mut self) -> Result<(), String> {
        let raw = self.add_name_buffer.trim();
        if raw.is_empty() {
            return Err("Enter a name.".to_string());
        }
        if raw.eq_ignore_ascii_case("default") || raw == ONE_DARK_PRO_ACTIVE {
            return Err("That name is reserved.".to_string());
        }
        if self.user_presets.contains_key(raw) {
            return Err("A preset with that name already exists.".to_string());
        }
        self.user_presets.insert(raw.to_string(), self.editing);
        self.wallpaper_by_name
            .insert(raw.to_string(), WallpaperSettings::default());
        self.selected          = PresetChoice::Custom(raw.to_string());
        self.editing_is_default = false;
        self.undo_snapshot     = self.editing;
        self.editing_wallpaper = self
            .wallpaper_by_name
            .get(raw)
            .cloned()
            .unwrap_or_default();
        self.undo_wallpaper   = self.editing_wallpaper.clone();
        self.add_name_buffer.clear();
        self.add_error          = None;
        self.name_prompt_open   = false;
        Ok(())
    }
}

fn preset_choice_from_active(s: &str) -> PresetChoice {
    match s {
        "default" => PresetChoice::Default,
        ONE_DARK_PRO_ACTIVE => PresetChoice::OneDarkPro,
        _ => PresetChoice::Custom(s.to_string()),
    }
}

fn commit_editing_palette_to_user_presets(state: &mut CustomizationState) {
    if let PresetChoice::Custom(n) = &state.selected {
        state.user_presets.insert(n.clone(), state.editing);
    }
}

fn save_active_preset_to_disk(state: &mut CustomizationState) -> Result<(), String> {
    let active: &str = match &state.selected {
        PresetChoice::Default => "default",
        PresetChoice::OneDarkPro => ONE_DARK_PRO_ACTIVE,
        PresetChoice::Custom(n) => n,
    };
    save_to_disk(
        active,
        &state.user_presets,
        &state.default_wallpaper,
        &state.wallpaper_by_name,
        state.vscode_style_chrome,
    )
}

/// merges `editing_wallpaper` into the correct map entry for the selected preset
fn sync_editing_wallpaper_into_state(state: &mut CustomizationState) {
    let mut w = state.editing_wallpaper.clone();
    w.clamp_alpha();
    state.editing_wallpaper = w.clone();
    match &state.selected {
        PresetChoice::Default => {
            state.default_wallpaper = w;
        }
        PresetChoice::OneDarkPro => {
            state
                .wallpaper_by_name
                .insert(ONE_DARK_PRO_ACTIVE.to_string(), w);
        }
        PresetChoice::Custom(n) => {
            state.wallpaper_by_name.insert(n.clone(), w);
        }
    }
}

fn after_apply_from_state(state: &CustomizationState) -> AfterCustomizationApply {
    AfterCustomizationApply {
        user_presets:         state.user_presets.clone(),
        choice:                state.selected.clone(),
        palette:              state.editing,
        default_wallpaper:     state.default_wallpaper.clone(),
        wallpaper_for_named:  state.wallpaper_by_name.clone(),
        vscode_style_chrome:  state.vscode_style_chrome,
    }
}

fn snapshot_from_state(state: &CustomizationState) -> SessionSnapshot {
    let color_active = match &state.selected {
        PresetChoice::Default => "default".to_string(),
        PresetChoice::OneDarkPro => ONE_DARK_PRO_ACTIVE.to_string(),
        PresetChoice::Custom(n) => n.clone(),
    };
    SessionSnapshot {
        color_palette:         state.editing,
        color_active,
        user_presets:          state.user_presets.clone(),
        default_wallpaper:     state.default_wallpaper.clone(),
        wallpaper_for_named:  state.wallpaper_by_name.clone(),
        vscode_style_chrome:   state.vscode_style_chrome,
    }
}

fn dismiss_revert(
    state:   &mut CustomizationState,
    on_commit: &mut impl FnMut(AfterCustomizationApply),
) {
    let Some(base) = state.session_baseline.take() else {
        state.open = false;
        return;
    };
    let choice = preset_choice_from_active(&base.color_active);
    on_commit(AfterCustomizationApply {
        user_presets:         base.user_presets.clone(),
        choice,
        palette:              base.color_palette,
        default_wallpaper:   base.default_wallpaper.clone(),
        wallpaper_for_named:  base.wallpaper_for_named.clone(),
        vscode_style_chrome:  base.vscode_style_chrome,
    });
    state.open = false;
    state.name_prompt_open   = false;
    state.add_error          = None;
    state.open_color_i       = None;
    state.add_name_buffer.clear();
}

fn vscode_dirty(state: &CustomizationState) -> bool {
    state.session_baseline.as_ref().is_some_and(|b| {
        b.vscode_style_chrome != state.vscode_style_chrome
    })
}

fn session_wallpaper_baseline(state: &CustomizationState) -> Option<WallpaperSettings> {
    let b = state.session_baseline.as_ref()?;
    Some(match &state.selected {
        PresetChoice::Default => b.default_wallpaper.clone(),
        PresetChoice::OneDarkPro => b
            .wallpaper_for_named
            .get(ONE_DARK_PRO_ACTIVE)
            .cloned()
            .unwrap_or_default(),
        PresetChoice::Custom(n) => b
            .wallpaper_for_named
            .get(n)
            .cloned()
            .unwrap_or_default(),
    })
}

fn wallpaper_dirty(state: &CustomizationState) -> bool {
    session_wallpaper_baseline(state).is_some_and(|w| w != state.editing_wallpaper)
}

const WALLPAPER_SLIDER_TRACK_MAX: f32 = 184.0;

fn wallpaper_presence_slider(
    ui:      &mut egui::Ui,
    enabled: bool,
    alpha:   &mut f32,
    id:      Id,
) {
    let w = (ui.available_width() - 4.0)
        .clamp(160.0, WALLPAPER_SLIDER_TRACK_MAX);
    let track_fill  = theme::main_central_fill().linear_multiply(0.55);
    let track_edge  = start_green().linear_multiply(0.30);

    let mut a = *alpha;
    Frame::new()
        .inner_margin(Margin::symmetric(9, 5))
        .fill(track_fill)
        .stroke(Stroke::new(1.0, track_edge))
        .corner_radius(CornerRadius::same(4))
        .show(ui, |ui| {
            ui.set_max_width(w + 20.0);
            ui.push_id(id, |ui| {
                ui.add_enabled_ui(enabled, |ui| {
                    ui.style_mut().visuals.slider_trailing_fill = true;
                    ui.style_mut().spacing.slider_width     = w;
                    ui.style_mut().spacing.item_spacing     = vec2(6.0, 4.0);
                    let sld = ui.add(
                        Slider::new(
                            &mut a,
                            MIN_WALLPAPER_ALPHA..=MAX_WALLPAPER_ALPHA,
                        )
                        .trailing_fill(true)
                        .text("Presence")
                        .custom_formatter(|n, _| {
                            let n = n as f32;
                            let span = MAX_WALLPAPER_ALPHA - MIN_WALLPAPER_ALPHA;
                            if span > 0.0 {
                                let t = (n - MIN_WALLPAPER_ALPHA) / span;
                                let pct = t.clamp(0.0, 1.0) * 100.0;
                                format!("{:.0}%", pct)
                            } else {
                                "—".to_string()
                            }
                        }),
                    );
                    sld.on_hover_text("How strong the image is (never fully covers the editor).");
                });
            });
        });
    *alpha = a;
}

fn wallpaper_01_slider(
    ui:      &mut egui::Ui,
    enabled: bool,
    value:   &mut f32,
    id:      Id,
    label:   &'static str,
    tip:     &'static str,
) {
    let w = (ui.available_width() * 0.5 - 8.0)
        .clamp(100.0, WALLPAPER_SLIDER_TRACK_MAX);
    let track_fill = theme::main_central_fill().linear_multiply(0.55);
    let track_edge = start_green().linear_multiply(0.30);
    let mut v      = *value;
    Frame::new()
        .inner_margin(Margin::symmetric(8, 5))
        .fill(track_fill)
        .stroke(Stroke::new(1.0, track_edge))
        .corner_radius(CornerRadius::same(4))
        .show(ui, |ui| {
            ui.set_max_width(w + 20.0);
            ui.push_id(id, |ui| {
                ui.add_enabled_ui(enabled, |ui| {
                    ui.style_mut().visuals.slider_trailing_fill = true;
                    ui.style_mut().spacing.slider_width     = w;
                    ui.style_mut().spacing.item_spacing     = vec2(4.0, 4.0);
                    let sld = ui.add(
                        Slider::new(&mut v, 0.0..=1.0)
                            .trailing_fill(true)
                            .text(label)
                            .custom_formatter(|n, _| format!("{:.0}%", n * 100.0)),
                    );
                    sld.on_hover_text(tip);
                });
            });
        });
    *value = v;
}

fn color_swatch_row(
    ui:          &mut egui::Ui,
    can_edit:    bool,
    i:           usize,
    open_here:   bool,
    label:       &str,
    c_val:       Color32,
    r:           u8,
    g:           u8,
    b:           u8,
    state:       &mut CustomizationState,
) {
    ui.horizontal_top(|ui| {
        let row_fill = ui.available_width();
        ui.set_min_width(row_fill);
        ui.add_sized(
            [124.0, 20.0],
            Label::new(
                RichText::new(label)
                    .size(10.0)
                    .monospace()
                    .color(start_green_dim()),
            )
            .wrap_mode(TextWrapMode::Extend),
        );
        let swatch_size  = vec2(34.0, 22.0);
        let swatch_sense = if can_edit {
            Sense::click()
        } else {
            Sense::hover()
        };
        let (sw_rect, sw) = {
            let (rect, response) = ui.allocate_at_least(swatch_size, swatch_sense);
            if ui.is_rect_visible(rect) {
                show_color_at(ui.painter(), c_val, rect);
            }
            (rect, response)
        };
        if can_edit {
            let sw = sw.on_hover_text("Click to open the color editor on the right");
            if sw.clicked() {
                let next = if open_here { None } else { Some(i) };
                if next.is_some() && next != state.open_color_i {
                    state.side_color_dock_bump = state.side_color_dock_bump.wrapping_add(1);
                }
                state.open_color_i = next;
            }
            ui.painter().rect_stroke(
                sw_rect,
                CornerRadius::same(3),
                Stroke::new(
                    1.0,
                    if open_here { start_green() } else { start_green_dim() },
                ),
                StrokeKind::Inside,
            );
        } else {
            ui.painter().rect_stroke(
                sw_rect,
                CornerRadius::same(3),
                Stroke::new(1.0, theme::sim_border()),
                StrokeKind::Inside,
            );
        }
        let rest = (ui.available_width() - 2.0).max(0.0);
        if rest > 4.0 {
            ui.allocate_ui_with_layout(
                vec2(rest, 44.0),
                Layout::right_to_left(Align::Min),
                |ui| {
                    ui.with_layout(Layout::top_down(Align::Max), |ui| {
                        ui.label(
                            RichText::new(format!("#{r:02X}{g:02X}{b:02X}"))
                                .monospace()
                                .size(11.0)
                                .color(start_green_dim()),
                        );
                        ui.add_space(2.0);
                        ui.label(
                            RichText::new(format!("{r:3} · {g:3} · {b:3}"))
                                .monospace()
                                .size(9.5)
                                .line_height(Some(12.0))
                                .color(start_green_dim()),
                        );
                    });
                },
            );
        }
    });
}

pub fn show_customization_overlay(
    ctx:  &egui::Context,
    state: &mut CustomizationState,
    on_commit: &mut impl FnMut(AfterCustomizationApply),
) {
    if !state.open {
        return;
    }

    let full  = ctx.screen_rect();
    let id    = egui::Id::new("customization_overlay");
    let mut close_now = false;

    // Backdrop: dim + click-outside (ignore while the name prompt is on top)
    egui::Area::new(id.with("dim"))
        .order(Order::Foreground)
        .interactable(true)
        .fixed_pos(full.min)
        .show(ctx, |ui| {
            let r = ui.allocate_response(full.size(), Sense::click());
            ui.painter().rect_filled(
                full,
                CornerRadius::ZERO,
                Color32::from_rgba_unmultiplied(0, 0, 0, 190),
            );
            if r.clicked() && !state.name_prompt_open {
                if state.open_color_i.is_some() {
                    state.open_color_i = None;
                } else {
                    close_now = true;
                }
            }
        });

    let max_win_w = (full.width() - 32.0).max(400.0);
    let cap_w     = 600.0_f32.min(max_win_w);
    let min_w     = 540.0_f32.min(cap_w);
    let custom_win = Window::new("##custom")
        .title_bar(false)
        .resizable(true)
        .order(Order::Foreground)
        .collapsible(false)
        .anchor(Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .min_width(min_w)
        .max_width(cap_w)
        .min_height(200.0)
        .max_height(full.height() - 32.0)
        .frame(
            Frame::NONE
                .fill(theme::sim_surface())
                .stroke(Stroke::new(1.0, start_green_dim()))
                .inner_margin(Margin::same(22))
                .corner_radius(CornerRadius::same(8)),
        )
        .show(ctx, |ui| {
            ui.vertical(|ui| {
                ui.label(
                    RichText::new("CUSTOMIZATION")
                        .size(20.0)
                        .color(start_green())
                        .strong(),
                );
                ui.add_space(10.0);

                ui.label(RichText::new("Preset").size(12.0).color(start_green_dim()));
                let prev   = state.selected_label();
                let mut pick: Option<String> = None;
                const PICK_ADD_NEW: &str = "ADD_NEW";
                let palette_locked = matches!(
                    state.selected,
                    PresetChoice::Default | PresetChoice::OneDarkPro
                );
                ui.horizontal(|ui| {
                    crate::theme::combo_box("preset_combo")
                        .selected_text(
                            RichText::new(&prev)
                                .monospace()
                                .size(12.0)
                                .color(if palette_locked {
                                    start_green_dim()
                                } else {
                                    start_green()
                                }),
                        )
                        .width(280.0)
                        .show_ui(ui, |ui| {
                            ui.set_min_width(260.0);
                            if ui
                                .selectable_label(
                                    state.selected == PresetChoice::Default,
                                    RichText::new("Default")
                                        .monospace()
                                        .size(12.0)
                                        .color(start_green()),
                                )
                                .clicked()
                            {
                                pick = Some("default".to_string());
                            }
                            if ui
                                .selectable_label(
                                    state.selected == PresetChoice::OneDarkPro,
                                    RichText::new("One Dark Pro")
                                        .monospace()
                                        .size(12.0)
                                        .color(start_green()),
                                )
                                .clicked()
                            {
                                pick = Some(ONE_DARK_PRO_ACTIVE.to_string());
                            }
                            for name in state.user_presets.keys() {
                                let sel = matches!(
                                    &state.selected,
                                    PresetChoice::Custom(n) if n == name
                                );
                                if ui
                                    .selectable_label(
                                        sel,
                                        RichText::new(name)
                                            .monospace()
                                            .size(12.0)
                                            .color(start_green()),
                                    )
                                    .clicked()
                                {
                                    pick = Some(name.clone());
                                }
                            }
                            if !state.user_presets.is_empty() {
                                ui.separator();
                            }
                            if ui
                                .selectable_label(
                                    false,
                                    RichText::new("Add new")
                                        .monospace()
                                        .size(12.0)
                                        .color(start_green_dim()),
                                )
                                .clicked()
                            {
                                pick = Some(PICK_ADD_NEW.to_string());
                            }
                        });
                });

                if let Some(key) = pick {
                    if key == PICK_ADD_NEW {
                        state.add_name_buffer.clear();
                        state.add_error   = None;
                        state.name_prompt_open = true;
                        state.name_prompt_request_focus = true;
                    } else if key == "default" {
                        state.open_color_i   = None;
                        state.selected          = PresetChoice::Default;
                        state.editing          = ThemePalette::DEFAULT;
                        state.editing_is_default = true;
                        state.undo_snapshot    = state.editing;
                        state.editing_wallpaper  = state.default_wallpaper.clone();
                        state.undo_wallpaper   = state.editing_wallpaper.clone();
                    } else if key == ONE_DARK_PRO_ACTIVE {
                        state.open_color_i = None;
                        state.selected = PresetChoice::OneDarkPro;
                        state.editing = ThemePalette::ONE_DARK_PRO;
                        state.editing_is_default = false;
                        state.undo_snapshot = state.editing;
                        state.editing_wallpaper = state
                            .wallpaper_by_name
                            .get(ONE_DARK_PRO_ACTIVE)
                            .cloned()
                            .unwrap_or_default();
                        state.undo_wallpaper = state.editing_wallpaper.clone();
                    } else if let Some(p) = state.user_presets.get(&key) {
                        let wpaper = state
                            .wallpaper_by_name
                            .get(&key)
                            .cloned()
                            .unwrap_or_default();
                        state.open_color_i   = None;
                        state.selected          = PresetChoice::Custom(key);
                        state.editing          = *p;
                        state.editing_is_default = false;
                        state.undo_snapshot    = state.editing;
                        state.editing_wallpaper = wpaper;
                        state.undo_wallpaper  = state.editing_wallpaper.clone();
                    }
                }

                ui.add_space(12.0);
                ui.separator();
                ui.add_space(8.0);

                ui.checkbox(
                    &mut state.vscode_style_chrome,
                    RichText::new("VS Code–style chrome")
                        .size(11.0)
                        .monospace()
                        .color(start_green_dim()),
                )
                .on_hover_text(
                    "Flat menu bar with sentence-case labels, subtler panel border, smaller control radii, \
                     light popover shadows, and slightly adjusted type scale. Applies on top of any color preset.",
                );
                ui.add_space(10.0);

                let can_edit = !palette_locked;
                let can_save = can_edit || wallpaper_dirty(state) || vscode_dirty(state);
                let path_short: String = {
                    let p = state.editing_wallpaper.path.as_str();
                    if p.is_empty() {
                        "—".to_string()
                    } else {
                        const MAX: usize = 36;
                        if p.len() > MAX {
                            let tail = p.len() - (MAX - 1);
                            format!("…{}", &p[tail..])
                        } else {
                            p.to_string()
                        }
                    }
                };
                ScrollArea::vertical()
                    .id_salt("custom_colors_scroll")
                    .max_width(cap_w - 44.0)
                    .max_height(460.0)
                    .auto_shrink([true, true])
                    .show(ui, |ui| {
                    // Stretch wallpaper + color rows to the scroll area (panel) width.
                    let fill_w = ui.available_width();
                    ui.set_min_width(fill_w);
                    ui.label(
                        RichText::new("Wallpaper")
                            .size(10.0)
                            .monospace()
                            .color(start_green_dim()),
                    );
                    ui.add_space(4.0);
                    ui.horizontal_top(|ui| {
                        ui.add_enabled_ui(state.editing_wallpaper.enabled, |ui| {
                            if ui
                                .button(
                                    RichText::new("Choose PNG…")
                                        .monospace()
                                        .size(12.0)
                                        .color(start_green_dim()),
                                )
                                .on_hover_text("Select a background image (PNG).")
                                .clicked()
                            {
                                if let Some(path) = rfd::FileDialog::new()
                                    .add_filter("PNG", &["png"])
                                    .pick_file()
                                {
                                    state.editing_wallpaper.path =
                                        path.to_string_lossy().to_string();
                                }
                            }
                        });
                        ui.add(
                            Label::new(
                                RichText::new(&path_short)
                                    .monospace()
                                    .size(9.0)
                                    .italics()
                                    .color(start_green_dim()),
                            )
                            .truncate(),
                        );
                    });
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        ui.checkbox(
                            &mut state.editing_wallpaper.enabled,
                            RichText::new("Enable wallpaper")
                                .size(11.0)
                                .monospace()
                                .color(start_green_dim()),
                        );
                        wallpaper_presence_slider(
                            ui,
                            state.editing_wallpaper.enabled,
                            &mut state.editing_wallpaper.alpha,
                            id.with("wp_alpha"),
                        );
                    });
                    if state.editing_wallpaper.enabled {
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            wallpaper_01_slider(
                                ui,
                                true,
                                &mut state.editing_wallpaper.blur,
                                id.with("wp_blur"),
                                "Blur",
                                "Applies a soft Gaussian-style blur to the full image (like many desktop rices).",
                            );
                            wallpaper_01_slider(
                                ui,
                                true,
                                &mut state.editing_wallpaper.corner_smooth,
                                id.with("wp_corner"),
                                "Corner",
                                "Rounds the wallpaper to match the app frame and softens the four corners with a second blur for a frosted edge.",
                            );
                        });
                    }
                    state.editing_wallpaper.clamp_alpha();
                    ui.add_space(10.0);
                    ui.separator();
                    ui.add_space(6.0);
                    for i in 0..ThemePalette::FIELD_COUNT {
                        let (label, c) = match theme_palette_field_mut(&mut state.editing, i) {
                            Some(x) => x,
                            None   => break,
                        };
                        let open_here  = state.open_color_i == Some(i);
                        let c_val = *c;
                        let (r, g, b) = (c.r(), c.g(), c.b());
                        let stripe    = (i % 2 == 1).then_some(ui.style().visuals.faint_bg_color);
                        if let Some(fill) = stripe {
                            egui::Frame::new()
                                .fill(fill)
                                .inner_margin(Margin::symmetric(4, 2))
                                .show(ui, |ui| {
                                    let w = ui.available_width();
                                    ui.set_min_width(w);
                                    color_swatch_row(
                                        ui,
                                        can_edit,
                                        i,
                                        open_here,
                                        label,
                                        c_val,
                                        r,
                                        g,
                                        b,
                                        state,
                                    );
                                });
                        } else {
                            color_swatch_row(
                                ui,
                                can_edit,
                                i,
                                open_here,
                                label,
                                c_val,
                                r,
                                g,
                                b,
                                state,
                            );
                        }
                    }
                    });

                ui.add_space(12.0);
                ui.vertical(|ui| {
                ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                    if ui
                        .add(
                            Button::new(
                                RichText::new("Apply")
                                    .monospace()
                                    .size(12.0)
                                    .color(start_green()),
                            ),
                        )
                        .on_hover_text(
                            "Apply the theme, write the active preset to the preset file, and update the app so it stays after restart.",
                        )
                        .clicked()
                    {
                        commit_editing_palette_to_user_presets(state);
                        sync_editing_wallpaper_into_state(state);
                        let _ = save_active_preset_to_disk(state);
                        on_commit(after_apply_from_state(state));
                        state.session_baseline  = Some(snapshot_from_state(state));
                        state.undo_snapshot     = state.editing;
                        state.undo_wallpaper   = state.editing_wallpaper.clone();
                        state.undo_vscode_chrome = state.vscode_style_chrome;
                    }
                });
                ui.add_space(8.0);
                ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                    if ui
                        .button(
                            RichText::new("Close")
                                .monospace()
                                .size(12.0)
                                .color(start_green_dim()),
                        )
                        .clicked()
                    {
                        close_now = true;
                    }
                    ui.add_space(8.0);
                    if ui
                        .add_enabled(
                            can_save,
                            Button::new(
                                RichText::new("Save")
                                    .monospace()
                                    .size(12.0)
                                    .color(if can_save {
                                        theme::on_accent_text()
                                    } else {
                                        theme::dim_gray()
                                    }),
                            )
                            .fill(if can_save {
                                start_green()
                            } else {
                                theme::disabled_panel()
                            })
                            .stroke(Stroke::new(
                                1.0,
                                if can_save {
                                    start_green_dim()
                                } else {
                                    theme::sim_border()
                                },
                            )),
                        )
                        .on_disabled_hover_text(
                            "No changes to save — edit colors, wallpaper, or VS Code–style chrome.",
                        )
                        .clicked()
                    {
                        if can_edit {
                            if let PresetChoice::Custom(name) = state.selected.clone() {
                                state.user_presets.insert(name.clone(), state.editing);
                                sync_editing_wallpaper_into_state(state);
                                let _ = save_to_disk(
                                    &name,
                                    &state.user_presets,
                                    &state.default_wallpaper,
                                    &state.wallpaper_by_name,
                                    state.vscode_style_chrome,
                                );
                                state.undo_snapshot  = state.editing;
                                state.undo_wallpaper = state.editing_wallpaper.clone();
                                state.undo_vscode_chrome = state.vscode_style_chrome;
                                state.session_baseline = Some(snapshot_from_state(state));
                            }
                        } else if wallpaper_dirty(state) || vscode_dirty(state) {
                            if wallpaper_dirty(state) {
                                sync_editing_wallpaper_into_state(state);
                            }
                            let app_active = state
                                .session_baseline
                                .as_ref()
                                .map(|b| b.color_active.as_str())
                                .unwrap_or("default");
                            let _ = save_to_disk(
                                app_active,
                                &state.user_presets,
                                &state.default_wallpaper,
                                &state.wallpaper_by_name,
                                state.vscode_style_chrome,
                            );
                            state.undo_snapshot = state.editing;
                            state.undo_wallpaper = state.editing_wallpaper.clone();
                            state.undo_vscode_chrome = state.vscode_style_chrome;
                            state.session_baseline = Some(snapshot_from_state(state));
                        }
                    }
                    ui.add_space(8.0);
                    if ui
                        .add(
                            Button::new(
                                RichText::new("Undo")
                                    .monospace()
                                    .size(12.0)
                                    .color(start_green()),
                            )
                            .fill(theme::sim_surface_lift())
                            .stroke(Stroke::new(1.0, theme::sim_border_bright())),
                        )
                        .clicked()
                    {
                        state.editing         = state.undo_snapshot;
                        state.editing_wallpaper = state.undo_wallpaper.clone();
                        state.vscode_style_chrome = state.undo_vscode_chrome;
                    }
                });
                });
            });
        });

    if let Some(ir) = custom_win {
        state.last_custom_window_rect = Some(ir.response.rect);
    }

    if let (Some(ci), Some(win_r)) = (state.open_color_i, state.last_custom_window_rect) {
        let margin   = 18.0f32;
        let panel_w  = 300.0f32;
        let dock_pos = pos2(win_r.right() - panel_w - margin, win_r.min.y + margin);
        let title    = PALETTE_SLIDER_LABELS.get(ci).copied().unwrap_or("Color");
        let side_id  = id
            .with("side_color_panel")
            .with(state.custom_open_generation)
            .with(state.side_color_dock_bump)
            .with(ci);
        let _side = Window::new(title)
            .id(side_id)
            .title_bar(true)
            .resizable(true)
            .order(Order::Foreground)
            .collapsible(false)
            .min_width(280.0)
            .default_size(vec2(panel_w, 420.0))
            .default_pos(dock_pos)
            .constrain_to(full)
            .frame(
                Frame::NONE
                    .fill(theme::sim_surface())
                    .stroke(Stroke::new(1.0, start_green_dim()))
                    .inner_margin(Margin::same(10))
                    .corner_radius(CornerRadius::same(8)),
            )
            .show(ctx, |ui| {
                ui.horizontal_top(|ui| {
                    ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                        if ui
                            .add(
                                Button::new(
                                    RichText::new("✕")
                                        .size(15.0)
                                        .line_height(Some(16.0))
                                        .monospace()
                                        .color(start_green_dim()),
                                )
                                .min_size([26.0, 22.0].into())
                                .frame(true),
                            )
                            .on_hover_text("Close color editor")
                            .clicked()
                        {
                            state.open_color_i = None;
                        }
                    });
                });
                ui.add_space(4.0);
                ui.spacing_mut().slider_width = 275.0;
                if let Some((_, c)) = theme_palette_field_mut(&mut state.editing, ci) {
                    color_picker_color32(ui, c, Alpha::Opaque);
                }
            });
    }

    if state.name_prompt_open {
        let prompt_id = id.with("name_prompt");
        egui::Area::new(prompt_id.with("dim"))
            .order(Order::Tooltip)
            .interactable(true)
            .fixed_pos(full.min)
            .show(ctx, |ui| {
                let r = ui.allocate_response(full.size(), Sense::click());
                ui.painter().rect_filled(
                    full,
                    CornerRadius::ZERO,
                    Color32::from_rgba_unmultiplied(0, 0, 0, 110),
                );
                if r.clicked() {
                    state.name_prompt_open = false;
                    state.add_error        = None;
                }
            });

        let mut confirm = false;
        let mut cancel  = false;
        Window::new("##name_preset")
            .title_bar(false)
            .resizable(false)
            .order(Order::Tooltip)
            .collapsible(false)
            .anchor(Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .frame(
                Frame::NONE
                    .fill(theme::sim_surface())
                    .stroke(Stroke::new(1.0, start_green_dim()))
                    .inner_margin(Margin::same(16))
                    .corner_radius(CornerRadius::same(8)),
            )
            .show(ctx, |ui| {
                ui.set_min_width(300.0);
                let te = TextEdit::singleline(&mut state.add_name_buffer)
                    .id(prompt_id.with("field"))
                    .hint_text(
                        RichText::new("Name your preset")
                            .color(theme::dim_gray())
                            .monospace()
                            .size(12.0),
                    )
                    .desired_width(260.0)
                    .font(egui::TextStyle::Monospace);
                let resp = ui.add(te);
                if state.name_prompt_request_focus {
                    resp.request_focus();
                    state.name_prompt_request_focus = false;
                }
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    if ui
                        .button(
                            RichText::new("Cancel")
                                .monospace()
                                .size(12.0)
                                .color(start_green_dim()),
                        )
                        .clicked()
                    {
                        cancel = true;
                    }
                    if ui
                        .add(
                            Button::new(
                                RichText::new("OK")
                                    .monospace()
                                    .size(12.0)
                                    .color(theme::on_accent_text()),
                            )
                            .fill(start_green())
                            .stroke(Stroke::new(1.0, start_green_dim())),
                        )
                        .clicked()
                    {
                        confirm = true;
                    }
                });
                if let Some(ref e) = state.add_error {
                    ui.add_space(6.0);
                    ui.label(RichText::new(e).size(10.5).color(theme::err_red()));
                }
            });

        if cancel {
            state.name_prompt_open = false;
            state.add_error        = None;
        } else if confirm {
            match state.apply_add_named_preset() {
                Ok(())  => {}
                Err(e) => state.add_error = Some(e),
            }
        }
    }

    if close_now {
        dismiss_revert(state, on_commit);
    } else if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        if state.name_prompt_open {
            state.name_prompt_open = false;
            state.add_error        = None;
        } else if state.open_color_i.is_some() {
            state.open_color_i = None;
        } else {
            dismiss_revert(state, on_commit);
        }
    }
}
