//! upload panel
use eframe::egui::{self, Button, Color32, Frame, Margin, RichText, Stroke, TextEdit, Ui};

use crate::avr::McuModel;
use crate::theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UploadAction {
    None,
    AssembleAndLink,
    UploadAvrdude,
}

pub fn scan_serial_ports() -> Vec<String> {
    let mut v = Vec::new();
    #[cfg(unix)]
    {
        if let Ok(rd) = std::fs::read_dir("/dev") {
            for e in rd.flatten() {
                let name = e.file_name().to_string_lossy().into_owned();
                let is_serial = name.starts_with("tty.usb")
                    || name.starts_with("cu.usb")
                    || name.starts_with("tty.wchusb")
                    || name.starts_with("tty.SLAB")
                    || name.starts_with("tty.usbserial")
                    || name.starts_with("ttyACM")
                    || name.starts_with("ttyUSB");
                if is_serial {
                    v.push(format!("/dev/{name}"));
                }
            }
        }
        v.sort();
        v.dedup();
    }
    #[cfg(windows)]
    {
        // COM enumeration would need extra APIs; users can type COM3 in custom path.
        let _ = &mut v;
    }
    v
}

pub fn show_upload_panel(
    ui: &mut Ui,
    hex_rel_path: &str,
    // workspace open — upload runs assemble first, then avrdude (hex need not exist yet).
    upload_enabled: bool,
    assembled_board: Option<McuModel>,
    programmer: &mut String,
    port: &mut String,
    port_custom: &mut bool,
    serial_ports: &[String],
    status_line: &str,
) -> UploadAction {
    let mut action = UploadAction::None;

    Frame::NONE
        .fill(theme::panel_over_wallpaper(ui.ctx(), theme::panel_deep()))
        .stroke(Stroke::new(0.75, theme::sim_border()))
        .inner_margin(Margin::same(10))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.set_min_height(ui.available_height());

            ui.label(
                RichText::new("[ UPLOAD ]")
                    .monospace()
                    .size(13.0)
                    .color(theme::start_green()),
            );
            ui.add_space(6.0);

            ui.label(
                RichText::new(format!("Output: {hex_rel_path} (replaced each build)"))
                    .monospace()
                    .size(10.5)
                    .color(theme::dim_gray()),
            );
            ui.add_space(10.0);

            if big_btn(ui, "Assemble and Link").clicked() {
                action = UploadAction::AssembleAndLink;
            }
            ui.add_space(8.0);

            ui.label(
                RichText::new("AVRDUDE")
                    .monospace()
                    .size(11.0)
                    .color(theme::start_green_dim()),
            );
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("-p")
                        .monospace()
                        .size(11.0)
                        .color(theme::dim_gray()),
                );
                let part_line = match assembled_board {
                    Some(m) => format!("{}  ({})", m.avrdude_part(), m.label()),
                    None => "—  (assemble with .board first)".to_string(),
                };
                ui.label(
                    RichText::new(part_line)
                        .monospace()
                        .size(11.0)
                        .color(theme::start_green()),
                );
            });
            if assembled_board == Some(McuModel::Atmega328P) {
                ui.label(
                    RichText::new("Uno built-in LED: PB5 — bitmask 0x20 (0x01 is PB0 / D8).")
                        .monospace()
                        .size(9.5)
                        .color(theme::dim_gray()),
                );
            }
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("-c")
                        .monospace()
                        .size(11.0)
                        .color(theme::dim_gray()),
                );
                ui.add(
                    TextEdit::singleline(programmer)
                        .desired_width(120.0)
                        .font(egui::TextStyle::Monospace),
                );
            });
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("-P")
                        .monospace()
                        .size(11.0)
                        .color(theme::dim_gray()),
                );
                let n = serial_ports.len();
                let custom_idx = n;
                let idx_in_list = serial_ports.iter().position(|p| p == port);
                let mut sel = if *port_custom || idx_in_list.is_none() {
                    custom_idx
                } else {
                    idx_in_list.unwrap_or(custom_idx)
                };

                let selected_label = if sel < n {
                    serial_ports[sel].as_str()
                } else if port.is_empty() {
                    "— Port —"
                } else {
                    "Custom path…"
                };

                theme::combo_box("upload_serial_port")
                    .selected_text(
                        RichText::new(selected_label)
                            .monospace()
                            .size(11.0),
                    )
                    .width(200.0)
                    .show_ui(ui, |ui| {
                        for (i, p) in serial_ports.iter().enumerate() {
                            ui.selectable_value(&mut sel, i, RichText::new(p).monospace());
                        }
                        ui.selectable_value(
                            &mut sel,
                            custom_idx,
                            RichText::new("Custom path…").monospace(),
                        );
                    });

                if sel < n {
                    *port = serial_ports[sel].clone();
                    *port_custom = false;
                } else {
                    *port_custom = true;
                }
            });

            if *port_custom {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(" ")
                            .monospace()
                            .size(11.0)
                            .color(theme::dim_gray()),
                    );
                    ui.add(
                        TextEdit::singleline(port)
                            .desired_width(220.0)
                            .font(egui::TextStyle::Monospace),
                    );
                });
            }

            ui.add_space(6.0);

            let can_upload = upload_enabled;
            let upload_resp = ui.add_enabled(
                can_upload,
                Button::new(
                    RichText::new("Upload Using AVRDUDE")
                        .monospace()
                        .size(12.0)
                        .color(if can_upload { Color32::BLACK } else { theme::dim_gray() }),
                )
                .fill(if can_upload {
                    theme::start_green()
                } else {
                    theme::disabled_panel()
                })
                .stroke(Stroke::new(1.0, if can_upload { theme::start_green() } else { theme::dim_gray() })),
            );
            if upload_resp.clicked() {
                action = UploadAction::UploadAvrdude;
            }
            ui.label(
                RichText::new("Upload rebuilds firmware.hex (assemble + link), then runs avrdude.")
                    .monospace()
                    .size(9.5)
                    .color(theme::dim_gray()),
            );

            #[cfg(not(target_os = "macos"))]
            {
                ui.add_space(8.0);
                ui.label(
                    RichText::new("Install avrdude and ensure it is on your PATH.")
                        .monospace()
                        .size(10.0)
                        .color(theme::dim_gray()),
                );
            }

            ui.add_space(10.0);
            ui.separator();
            ui.add_space(6.0);

            ui.label(
                RichText::new("STATUS")
                    .monospace()
                    .size(11.0)
                    .color(theme::start_green_dim()),
            );
            ui.add_space(6.0);
            let status_h = (ui.available_height() - 2.0).max(48.0);
            egui::ScrollArea::vertical()
                .id_salt("upload_status_scroll")
                .auto_shrink([false, false])
                .max_height(status_h)
                .show(ui, |ui| {
                    ui.label(
                        RichText::new(if status_line.is_empty() {
                            "(idle)"
                        } else {
                            status_line
                        })
                        .monospace()
                        .size(10.5)
                        .color(
                            if status_line.contains("Error") || status_line.contains("not found") {
                                theme::focus()
                            } else {
                                theme::start_green()
                            },
                        ),
                    );
                });

        });

    action
}

fn big_btn(ui: &mut Ui, label: &str) -> egui::Response {
    ui.add(
        Button::new(
            RichText::new(label)
                .monospace()
                .size(12.0)
                .color(Color32::BLACK),
        )
        .fill(theme::start_green_dim())
        .stroke(Stroke::new(1.0, theme::start_green())),
    )
}
