//! toolbar_basic

use std::path::Path;
use std::sync::Arc;

use eframe::egui::{
    menu, Align, FontFamily, FontId, Frame, Layout, Margin, RichText,
    Stroke, Ui,
};

use crate::avr::McuModel;
use crate::theme::{self, ChromeProfile};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ToolbarAction {
    None,
    Save,
    SaveAll,
    NewFile,
    NewDir,
    OpenFolder,
    AddFileToProject,
    SimTogglePanel,
    PeripheralsTogglePanel,
    WaveformsTogglePanel,
    UartTogglePanel,
    UploadTogglePanel,
    DocsFlashLocations,
    HelpersWordHelper,
    HelpersCycleHelper,
    HelpersCostAnalysis,
    Customization,
}

fn title_font(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name(Arc::from("fm_title")))
}

fn toolbar_is_vscode_style() -> bool {
    matches!(theme::chrome_profile(), ChromeProfile::VsCodeStyle)
}

/// Primary label on the menu bar (Orbitron + accent vs VS Code–style body + fg).
fn menubar_title_rt(odp: bool, label: &str) -> RichText {
    if odp {
        RichText::new(label)
            .font(FontId::new(15.0, FontFamily::Proportional))
            .color(theme::text_primary())
    } else {
        RichText::new(label)
            .font(title_font(18.0))
            .color(theme::start_green())
    }
}

/// Flat toggles: SIM ▪ vs compact “Sim ·”.
fn panel_toggle_caption(odp: bool, short: &str, on: bool) -> String {
    if odp {
        if on {
            format!("{short} ·")
        } else {
            short.to_string()
        }
    } else if on {
        format!("{short} ▪")
    } else {
        short.to_string()
    }
}

pub fn show_toolbar(
    ui:                  &mut Ui,
    active_file:         Option<&Path>,
    workspace_root:      &Path,
    is_dirty:            bool,
    sim_visible:        bool,
    peripherals_visible: bool,
    waveforms_visible:   bool,
    uart_visible:        bool,
    upload_visible:      bool,
    helpers_visible:    bool,
    assembled_board:   Option<McuModel>,
) -> ToolbarAction {
    let mut action = ToolbarAction::None;
    let odp = toolbar_is_vscode_style();

    let bar_fill = if odp {
        theme::panel_over_wallpaper(ui.ctx(), theme::panel_lift())
    } else {
        theme::panel_over_wallpaper(ui.ctx(), theme::panel_mid())
    };
    let bar_stroke = if odp {
        Stroke::new(1.0, theme::sim_border())
    } else {
        Stroke::new(1.0, theme::start_green_dim())
    };
    let bar_margin = if odp {
        Margin::symmetric(8, 5)
    } else {
        Margin::symmetric(10, 6)
    };

    Frame::NONE
        .fill(bar_fill)
        .stroke(bar_stroke)
        .inner_margin(bar_margin)
        .show(ui, |ui| {
            menu::bar(ui, |ui| {
                ui.menu_button(
                    menubar_title_rt(odp, if odp { "File" } else { "FILE" }),
                    |ui| {
                        theme::apply_dropdown_menu_style(ui);

                        if ui.button("Save").clicked() {
                            action = ToolbarAction::Save;
                            ui.close_menu();
                        }
                        if ui.button("Save all").clicked() {
                            action = ToolbarAction::SaveAll;
                            ui.close_menu();
                        }

                        ui.separator();

                        if ui.button(if odp { "New file…" } else { "New file" }).clicked() {
                            action = ToolbarAction::NewFile;
                            ui.close_menu();
                        }
                        if ui.button(if odp { "New folder…" } else { "New dir" }).clicked() {
                            action = ToolbarAction::NewDir;
                            ui.close_menu();
                        }

                        ui.separator();

                        if ui
                            .button(if odp {
                                "Open folder…"
                            } else {
                                "Open folder"
                            })
                            .clicked()
                        {
                            action = ToolbarAction::OpenFolder;
                            ui.close_menu();
                        }
                        if ui
                            .button(if odp {
                                "Add file to workspace…"
                            } else {
                                "Add file to project"
                            })
                            .clicked()
                        {
                            action = ToolbarAction::AddFileToProject;
                            ui.close_menu();
                        }

                        ui.separator();

                        if ui
                            .button(if odp {
                                "Theme…"
                            } else {
                                "Customization…"
                            })
                            .clicked()
                        {
                            action = ToolbarAction::Customization;
                            ui.close_menu();
                        }
                    },
                );

                let sim_label = panel_toggle_caption(odp, if odp { "Sim" } else { "SIM" }, sim_visible);
                if ui
                    .add(eframe::egui::Button::new(
                        menubar_title_rt(odp, sim_label.as_str()),
                    )
                    .frame(false))
                    .clicked()
                {
                    action = ToolbarAction::SimTogglePanel;
                }

                let periph_label = panel_toggle_caption(
                    odp,
                    if odp { "Periph" } else { "PERIPH" },
                    peripherals_visible,
                );
                if ui
                    .add(eframe::egui::Button::new(
                        menubar_title_rt(odp, periph_label.as_str()),
                    )
                    .frame(false))
                    .clicked()
                {
                    action = ToolbarAction::PeripheralsTogglePanel;
                }

                let wf_label = panel_toggle_caption(
                    odp,
                    if odp { "Waveforms" } else { "WAVEFORMS" },
                    waveforms_visible,
                );
                if ui
                    .add(eframe::egui::Button::new(
                        menubar_title_rt(odp, wf_label.as_str()),
                    )
                    .frame(false))
                    .clicked()
                {
                    action = ToolbarAction::WaveformsTogglePanel;
                }

                let uart_label =
                    panel_toggle_caption(odp, if odp { "UART" } else { "UART" }, uart_visible);
                if ui
                    .add(eframe::egui::Button::new(
                        menubar_title_rt(odp, uart_label.as_str()),
                    )
                    .frame(false))
                    .clicked()
                {
                    action = ToolbarAction::UartTogglePanel;
                }

                let upload_label = panel_toggle_caption(
                    odp,
                    if odp { "Upload" } else { "UPLOAD" },
                    upload_visible,
                );
                if ui
                    .add(eframe::egui::Button::new(
                        menubar_title_rt(odp, upload_label.as_str()),
                    )
                    .frame(false))
                    .clicked()
                {
                    action = ToolbarAction::UploadTogglePanel;
                }

                ui.menu_button(
                    menubar_title_rt(odp, if odp { "Help" } else { "DOCS" }),
                    |ui| {
                        theme::apply_dropdown_menu_style(ui);
                        if ui
                            .button(if odp { "Flash memory map" } else { "Flash locations" })
                            .clicked()
                        {
                            action = ToolbarAction::DocsFlashLocations;
                            ui.close_menu();
                        }
                    },
                );

                let helpers_label =
                    panel_toggle_caption(odp, if odp { "Helpers" } else { "HELPERS" }, helpers_visible);
                ui.menu_button(menubar_title_rt(odp, helpers_label.as_str()), |ui| {
                    theme::apply_dropdown_menu_style(ui);
                    if ui.button("Word helper").clicked() {
                        action = ToolbarAction::HelpersWordHelper;
                        ui.close_menu();
                    }
                    if ui.button("Cycle helper").clicked() {
                        action = ToolbarAction::HelpersCycleHelper;
                        ui.close_menu();
                    }
                    if ui.button("Cost analysis").clicked() {
                        action = ToolbarAction::HelpersCostAnalysis;
                        ui.close_menu();
                    }
                });

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let mut label = active_file
                        .and_then(|path| path.file_name())
                        .map(|name| name.to_string_lossy().to_string())
                        .unwrap_or_else(|| "(unsaved)".to_string());
                    if is_dirty {
                        label.push_str(" *");
                    }

                    let (file_color, file_size) = if odp {
                        (theme::text_primary(), 14.0_f32)
                    } else {
                        (theme::start_green(), 14.0_f32)
                    };
                    ui.label(
                        RichText::new(label)
                            .monospace()
                            .color(file_color)
                            .size(file_size),
                    );
                    ui.add_space(12.0);

                    let meta_color = if odp {
                        theme::dim_gray()
                    } else {
                        theme::start_green_dim()
                    };
                    ui.label(
                        RichText::new(
                            assembled_board
                                .map(|m| m.label().to_string())
                                .unwrap_or_else(|| "—".to_string()),
                        )
                        .monospace()
                        .color(meta_color)
                        .size(if odp { 12.0 } else { 11.5 }),
                    );
                    ui.add_space(10.0);
                    ui.label(
                        RichText::new(workspace_root.display().to_string())
                            .monospace()
                            .color(meta_color)
                            .size(if odp { 12.5 } else { 12.0 }),
                    );
                });
            });
        });

    action
}
