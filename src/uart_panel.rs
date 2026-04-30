//! virtual USART + optional USB serial to a programmed device

use std::io::Write;
use std::time::{Duration, Instant};

use eframe::egui::{
    self, Align, Button, CornerRadius, DragValue, Frame, Layout, Margin,
    RichText, ScrollArea, Slider, Stroke, TextEdit, Ui,
};
use serialport::SerialPort;

use crate::avr::cpu::Cpu;
use crate::real_uart;
use crate::avr::McuModel;
use crate::sim_panel::{show_sim_machine_status_row, show_sim_sticky_controls, SimAction, SpeedLimitState};
use crate::theme;

const USART0_TERM_W: f32 = 280.0;
const USART0_TERM_H: f32 = 180.0;
const USART1_TERM_W: f32 = 260.0;
const USART1_TERM_H: f32 = 120.0;
const TERM_MIN_OUTER_H: f32 = 52.0;
const HOST_RX_SEND_MAX_W: f32 = 200.0;

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum UartSendLineEnding {
    #[default]
    None,
    /// `\n`
    Newline,
    /// `\r`
    CarriageReturn,
    /// `\r\n`
    BothNlAndCr,
}

impl UartSendLineEnding {
    fn label(self) -> &'static str {
        match self {
            UartSendLineEnding::None => "No line ending",
            UartSendLineEnding::Newline => "Newline",
            UartSendLineEnding::CarriageReturn => "Carriage return",
            UartSendLineEnding::BothNlAndCr => "Both NL & CR",
        }
    }

    fn trailing_bytes(self) -> &'static [u8] {
        match self {
            UartSendLineEnding::None => &[],
            UartSendLineEnding::Newline => b"\n",
            UartSendLineEnding::CarriageReturn => b"\r",
            UartSendLineEnding::BothNlAndCr => b"\r\n",
        }
    }

    fn push_after_payload(self, cpu: &mut Cpu, port: u8) {
        match self {
            UartSendLineEnding::None => {}
            UartSendLineEnding::Newline => cpu.usart_rx_host_push(port, b'\n'),
            UartSendLineEnding::CarriageReturn => cpu.usart_rx_host_push(port, b'\r'),
            UartSendLineEnding::BothNlAndCr => {
                cpu.usart_rx_host_push(port, b'\r');
                cpu.usart_rx_host_push(port, b'\n');
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum UartBackend {
    #[default]
    /// Data from the in-app AVR simulation only.
    Simulator,
    /// Bytes to/from a real serial port (same device path as avrdude `-P`).
    UsbSerial,
}

pub struct UartPanelState {
    pub line0: String,
    pub line1: String,
    pub rx0_scroll: String,
    pub rx0_partial: String,
    pub rx0_stab:    LineStabilizer,
    pub rx1_scroll:  String,
    pub rx1_partial: String,
    pub rx1_stab:    LineStabilizer,
    pub rx0_pending_cr: bool,
    pub rx1_pending_cr: bool,
    pub send_line_ending: UartSendLineEnding,
    pub backend:          UartBackend,
    pub hardware_baud: u32,
    pub display_hold_ms: u32,
    pub rx0_frozen: String,
    rx0_frozen_at: Option<Instant>,
    pub rx1_frozen: String,
    rx1_frozen_at: Option<Instant>,
}

impl UartPanelState {
    #[allow(dead_code)]
    pub fn rx0_display(&self) -> String {
        let mut s = self.rx0_scroll.clone();
        s.push_str(&self.rx0_partial);
        s
    }

    #[allow(dead_code)]
    pub fn rx1_display(&self) -> String {
        let mut s = self.rx1_scroll.clone();
        s.push_str(&self.rx1_partial);
        s
    }

    /// Snapshot passed to the terminal [`Label`] (throttled by [`refresh_uart_display_throttle`]).
    pub fn rx0_shown(&self) -> &str {
        self.rx0_frozen.as_str()
    }

    pub fn rx1_shown(&self) -> &str {
        self.rx1_frozen.as_str()
    }

    pub fn clear_throttled_display(&mut self) {
        self.rx0_frozen.clear();
        self.rx0_frozen_at = None;
        self.rx1_frozen.clear();
        self.rx1_frozen_at = None;
    }

    /// Clear device→host scrollback and in-progress line (both USARTs). Does not clear host send drafts.
    pub fn clear_monitor(&mut self) {
        self.rx0_scroll.clear();
        self.rx0_partial.clear();
        self.rx0_stab.reset();
        self.rx0_pending_cr = false;
        self.rx1_scroll.clear();
        self.rx1_partial.clear();
        self.rx1_stab.reset();
        self.rx1_pending_cr = false;
        self.clear_throttled_display();
    }
}

impl Default for UartPanelState {
    fn default() -> Self {
        Self {
            line0: String::new(),
            line1: String::new(),
            rx0_scroll:  String::new(),
            rx0_partial: String::new(),
            rx0_stab:    LineStabilizer::default(),
            rx1_scroll:  String::new(),
            rx1_partial: String::new(),
            rx1_stab:    LineStabilizer::default(),
            rx0_pending_cr: false,
            rx1_pending_cr: false,
            send_line_ending: UartSendLineEnding::Newline,
            backend:          UartBackend::Simulator,
            hardware_baud:    115200,
            display_hold_ms:  100,
            rx0_frozen:       String::new(),
            rx0_frozen_at:    None,
            rx1_frozen:       String::new(),
            rx1_frozen_at:    None,
        }
    }
}

pub fn refresh_uart_display_throttle(state: &mut UartPanelState) {
    let now = Instant::now();

    let truth0 = {
        let mut s = state.rx0_scroll.clone();
        s.push_str(&state.rx0_partial);
        s
    };
    if state.display_hold_ms == 0 {
        state.rx0_frozen = truth0;
        state.rx0_frozen_at = Some(now);
    } else {
        let iv = Duration::from_millis(state.display_hold_ms as u64);
        let update = match state.rx0_frozen_at {
            None => true,
            Some(ts) if now.duration_since(ts) >= iv => true,
            _ => false,
        };
        if update {
            state.rx0_frozen = truth0;
            state.rx0_frozen_at = Some(now);
        }
    }

    let truth1 = {
        let mut s = state.rx1_scroll.clone();
        s.push_str(&state.rx1_partial);
        s
    };
    if state.display_hold_ms == 0 {
        state.rx1_frozen = truth1;
        state.rx1_frozen_at = Some(now);
    } else {
        let iv = Duration::from_millis(state.display_hold_ms as u64);
        let update = match state.rx1_frozen_at {
            None => true,
            Some(ts) if now.duration_since(ts) >= iv => true,
            _ => false,
        };
        if update {
            state.rx1_frozen = truth1;
            state.rx1_frozen_at = Some(now);
        }
    }
}

const HW_BAUD_CHOICES: &[u32] =
    &[9600, 19200, 38400, 57600, 115200, 230400, 500000, 1_000_000];

#[derive(Clone, Debug)]
pub struct LineStabilizer {
    pub enabled: bool,
    pub min_interval_ms: u32,
    last_commit:     Option<Instant>,
    last_was_rapid:  bool,
}

impl Default for LineStabilizer {
    fn default() -> Self {
        Self {
            enabled: true,
            min_interval_ms: 80,
            last_commit: None,
            last_was_rapid: false,
        }
    }
}

impl LineStabilizer {
    pub fn reset(&mut self) {
        self.last_commit = None;
        self.last_was_rapid = false;
    }
}

fn remove_last_scroll_line(scroll: &mut String) {
    if scroll.is_empty() {
        return;
    }
    if !scroll.ends_with('\n') {
        return;
    }
    scroll.pop();
    match scroll.rfind('\n') {
        Some(i) => scroll.truncate(i + 1),
        None => scroll.clear(),
    }
}

fn trim_scroll_max(scroll: &mut String) {
    const MAX: usize = 96_000;
    if scroll.len() <= MAX {
        return;
    }
    let over = scroll.len() - MAX;
    let start = scroll
        .char_indices()
        .map(|(i, _)| i)
        .find(|&i| i >= over)
        .unwrap_or(0);
    scroll.drain(..start);
}

fn trim_combined(scroll: &mut String, partial: &str) {
    const MAX: usize = 96_000;
    let total = scroll.len() + partial.len();
    if total <= MAX {
        return;
    }
    let over = total - MAX;
    if scroll.len() >= over {
        let start = scroll
            .char_indices()
            .map(|(i, _)| i)
            .find(|&i| i >= over)
            .unwrap_or(0);
        scroll.drain(..start);
    } else {
        scroll.clear();
    }
}

fn commit_line(
    scroll: &mut String,
    line: &mut String,
    stab: &mut LineStabilizer,
    line_idx_in_feed: usize,
) {
    let content = std::mem::take(line);
    if !stab.enabled {
        scroll.push_str(&content);
        scroll.push('\n');
        stab.last_commit = Some(Instant::now());
        stab.last_was_rapid = false;
        trim_scroll_max(scroll);
        return;
    }

    if line_idx_in_feed > 0 {
        remove_last_scroll_line(scroll);
        scroll.push_str(&content);
        scroll.push('\n');
        stab.last_commit = Some(Instant::now());
        stab.last_was_rapid = true;
        trim_scroll_max(scroll);
        return;
    }

    let now = Instant::now();
    let iv = Duration::from_millis(stab.min_interval_ms.clamp(16, 2000) as u64);

    match stab.last_commit {
        None => {
            scroll.push_str(&content);
            scroll.push('\n');
            stab.last_commit = Some(now);
            stab.last_was_rapid = false;
        }
        Some(t) => {
            let elapsed = now.duration_since(t);
            if elapsed >= iv {
                scroll.push_str(&content);
                scroll.push('\n');
                stab.last_commit = Some(now);
                stab.last_was_rapid = false;
            } else if stab.last_was_rapid {
                remove_last_scroll_line(scroll);
                scroll.push_str(&content);
                scroll.push('\n');
                stab.last_commit = Some(now);
            } else {
                scroll.push_str(&content);
                scroll.push('\n');
                stab.last_commit = Some(now);
                stab.last_was_rapid = true;
            }
        }
    }
    trim_scroll_max(scroll);
}

fn push_printable_or_escape(partial: &mut String, b: u8) {
    match b {
        0x20..=0x7E => {
            partial.push(b as char);
        }
        _ => {
            partial.push_str(&format!("\\x{b:02X}"));
        }
    }
}

fn feed_bytes_into_rx(
    scroll: &mut String,
    partial: &mut String,
    stab: &mut LineStabilizer,
    bytes: &[u8],
    line_in_feed: &mut usize,
    pending_cr: &mut bool,
) {
    let mut i = 0usize;
    while i < bytes.len() {
        if *pending_cr {
            *pending_cr = false;
            match bytes[i] {
                b'\n' => {
                    let idx = *line_in_feed;
                    *line_in_feed += 1;
                    commit_line(scroll, partial, stab, idx);
                    i += 1;
                    continue;
                }
                b'\r' => {
                    partial.clear();
                    *pending_cr = true;
                    i += 1;
                    continue;
                }
                b => {
                    partial.clear();
                    push_printable_or_escape(partial, b);
                    i += 1;
                    continue;
                }
            }
        }

        if bytes[i] == b'\r' && bytes.get(i + 1) == Some(&b'\n') {
            let idx = *line_in_feed;
            *line_in_feed += 1;
            commit_line(scroll, partial, stab, idx);
            i += 2;
            continue;
        }

        match bytes[i] {
            b'\r' => {
                *pending_cr = true;
                i += 1;
            }
            b'\n' => {
                let idx = *line_in_feed;
                *line_in_feed += 1;
                commit_line(scroll, partial, stab, idx);
                i += 1;
            }
            b => {
                push_printable_or_escape(partial, b);
                i += 1;
            }
        }
    }
    trim_combined(scroll, partial.as_str());
}

pub fn append_uart_tx_to_scrollback(cpu: &mut Cpu, model: McuModel, state: &mut UartPanelState) -> usize {
    let mut n = 0usize;
    let mut drain0 = Vec::new();
    cpu.usart_drain_tx_to_host(0, &mut drain0);
    n += drain0.len();
    let mut line_feed = 0usize;
    feed_bytes_into_rx(
        &mut state.rx0_scroll,
        &mut state.rx0_partial,
        &mut state.rx0_stab,
        &drain0,
        &mut line_feed,
        &mut state.rx0_pending_cr,
    );
    if model == McuModel::Atmega128A {
        let mut drain1 = Vec::new();
        cpu.usart_drain_tx_to_host(1, &mut drain1);
        n += drain1.len();
        let mut line_feed1 = 0usize;
        feed_bytes_into_rx(
            &mut state.rx1_scroll,
            &mut state.rx1_partial,
            &mut state.rx1_stab,
            &drain1,
            &mut line_feed1,
            &mut state.rx1_pending_cr,
        );
    }
    n
}

pub fn poll_hardware_serial(port: &mut dyn SerialPort, scratch: &mut [u8], state: &mut UartPanelState) -> usize {
    let mut total = 0usize;
    let mut line_in_feed = 0usize;
    loop {
        match port.read(scratch) {
            Ok(0) => break,
            Ok(n) => {
                feed_bytes_into_rx(
                    &mut state.rx0_scroll,
                    &mut state.rx0_partial,
                    &mut state.rx0_stab,
                    &scratch[..n],
                    &mut line_in_feed,
                    &mut state.rx0_pending_cr,
                );
                total += n;
            }
            Err(e)
                if e.kind() == std::io::ErrorKind::TimedOut
                    || e.kind() == std::io::ErrorKind::WouldBlock => break,
            Err(_) => break,
        }
    }
    total
}

pub fn show_uart_panel(
    ui:                 &mut Ui,
    cpu:                &mut Cpu,
    model:              McuModel,
    state:              &mut UartPanelState,
    assembled_board:    Option<McuModel>,
    auto_running:       bool,
    ips:                f64,
    speed_limit:        &mut SpeedLimitState,
    serial_ports:       &[String],
    upload_port:        &mut String,
    upload_port_custom: &mut bool,
    uart_serial:        &mut Option<Box<dyn SerialPort>>,
    uart_serial_error:  &mut Option<String>,
) -> SimAction {
    let mut action = SimAction::None;

    Frame::NONE
        .fill(theme::panel_over_wallpaper(ui.ctx(), theme::panel_deep()))
        .stroke(Stroke::new(0.75, theme::sim_border()))
        .inner_margin(Margin::same(10))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());

            let title = match assembled_board {
                Some(m) => format!("[ UART  {} ]", m.label()),
                None => "[ UART ]".to_string(),
            };
            ui.label(
                RichText::new(title)
                    .monospace()
                    .size(13.0)
                    .color(theme::accent()),
            );
            ui.add_space(6.0);

            let prev_backend = state.backend;
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("Source:")
                        .monospace()
                        .size(10.5)
                        .color(theme::accent_dim()),
                );
                ui.radio_value(&mut state.backend, UartBackend::Simulator, "Simulator");
                ui.radio_value(&mut state.backend, UartBackend::UsbSerial, "USB serial");
            });
            if prev_backend != state.backend {
                *uart_serial = None;
                uart_serial_error.take();
            }

            ui.add_space(4.0);
            match state.backend {
                UartBackend::Simulator => {
                    show_sim_machine_status_row(ui, cpu);
                }
                UartBackend::UsbSerial => {
                    ui.label(
                        RichText::new(
                            "Uses the same serial device as Upload (−P). Disconnect here before running avrdude, or use Upload (it closes the monitor automatically). Baud must match firmware (UBRR).",
                        )
                        .monospace()
                        .size(9.5)
                        .color(theme::accent_dim()),
                    );
                }
            }

            ui.add_space(6.0);

            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("Line ending:")
                        .monospace()
                        .size(10.5)
                        .color(theme::accent_dim()),
                );
                line_ending_combo(ui, state);
            });
            ui.add_space(4.0);

            ui.horizontal(|ui| {
                let ch = ui.checkbox(
                    &mut state.rx0_stab.enabled,
                    RichText::new("Stabilize fast line rate")
                        .monospace()
                        .size(10.5),
                );
                if ch.changed() {
                    state.rx1_stab.enabled = state.rx0_stab.enabled;
                }
                ui.add_enabled_ui(state.rx0_stab.enabled, |ui| {
                    ui.label(
                        RichText::new("gap")
                            .monospace()
                            .size(10.0)
                            .color(theme::accent_dim()),
                    );
                    let mut ms = state.rx0_stab.min_interval_ms;
                    let dv = ui.add(
                        DragValue::new(&mut ms)
                            .range(16..=500)
                            .suffix(" ms"),
                    );
                    if dv.changed() {
                        state.rx0_stab.min_interval_ms = ms;
                        state.rx1_stab.min_interval_ms = ms;
                    }
                });
            });
            ui.label(
                RichText::new(
                    "Time gap: rapid *successive* reads (same wall clock) coalesce. Extra lines in one USB read are merged to the latest line so kernel buffering does not erase the scroll.",
                )
                .monospace()
                .size(9.0)
                .color(theme::accent_dim()),
            );
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("Display hold")
                        .monospace()
                        .size(10.5)
                        .color(theme::accent_dim()),
                );
                ui.add(
                    Slider::new(&mut state.display_hold_ms, 0..=1000)
                        .suffix(" ms")
                        .text("min time between new on-screen values"),
                );
            });
            ui.label(
                RichText::new(
                    "0 ms = show every frame. Larger values cap how often the terminal text can change (reduces flicker and layout jump).",
                )
                .monospace()
                .size(9.0)
                .color(theme::accent_dim()),
            );
            ui.add_space(4.0);

            ScrollArea::vertical()
                .id_salt("uart_panel_body")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    match state.backend {
                        UartBackend::Simulator => {
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new("USART0 (device TX)")
                                        .monospace()
                                        .size(11.0)
                                        .color(theme::accent_dim()),
                                );
                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    if ui
                                        .small_button(
                                            RichText::new("Clear monitor")
                                                .monospace()
                                                .size(10.0)
                                                .color(theme::accent_dim()),
                                        )
                                        .clicked()
                                    {
                                        state.clear_monitor();
                                    }
                                });
                            });
                            ui.add_space(2.0);
                            terminal_scroll(
                                ui,
                                "uart_term0",
                                state.rx0_shown(),
                                USART0_TERM_W,
                                USART0_TERM_H,
                            );

                            ui.add_space(6.0);
                            uart_send_row(
                                ui,
                                cpu,
                                "Host → USART0 RX",
                                &mut state.line0,
                                0,
                                state.send_line_ending,
                            );

                            if model == McuModel::Atmega128A {
                                ui.add_space(12.0);
                                ui.horizontal(|ui| {
                                    ui.label(
                                        RichText::new("USART1 (device TX)")
                                            .monospace()
                                            .size(11.0)
                                            .color(theme::accent_dim()),
                                    );
                                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                        if ui
                                            .small_button(
                                                RichText::new("Clear monitor")
                                                    .monospace()
                                                    .size(10.0)
                                                    .color(theme::accent_dim()),
                                            )
                                            .clicked()
                                        {
                                            state.clear_monitor();
                                        }
                                    });
                                });
                                ui.add_space(2.0);
                                terminal_scroll(
                                    ui,
                                    "uart_term1",
                                    state.rx1_shown(),
                                    USART1_TERM_W,
                                    USART1_TERM_H,
                                );
                                ui.add_space(6.0);
                                uart_send_row(
                                    ui,
                                    cpu,
                                    "Host → USART1 RX",
                                    &mut state.line1,
                                    1,
                                    state.send_line_ending,
                                );
                            }
                        }
                        UartBackend::UsbSerial => {
                            show_uart_usb_section(
                                ui,
                                state,
                                serial_ports,
                                upload_port,
                                upload_port_custom,
                                uart_serial,
                                uart_serial_error,
                            );
                        }
                    }
                });

            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui
                        .small_button(
                            RichText::new("Clear serial logs")
                                .monospace()
                                .size(10.0)
                                .color(theme::accent_dim()),
                        )
                        .clicked()
                    {
                        state.clear_monitor();
                    }
                });
            });

            ui.add_space(4.0);
            ui.separator();
            ui.add_space(4.0);

            let sticky = show_sim_sticky_controls(
                ui,
                auto_running,
                ips,
                speed_limit,
                "ASSEMBLE  (from editor)",
                "uart_ips_unit",
            );
            if sticky != SimAction::None {
                action = sticky;
            }
        });

    action
}

fn show_uart_usb_section(
    ui:                 &mut Ui,
    state:              &mut UartPanelState,
    serial_ports:       &[String],
    upload_port:        &mut String,
    upload_port_custom: &mut bool,
    uart_serial:        &mut Option<Box<dyn SerialPort>>,
    uart_serial_error:  &mut Option<String>,
) {
    ui.horizontal(|ui| {
        ui.label(
            RichText::new("Port (−P)")
                .monospace()
                .size(10.5)
                .color(theme::accent_dim()),
        );
        let n = serial_ports.len();
        let custom_idx = n;
        let idx_in_list = serial_ports.iter().position(|p| p == upload_port.as_str());
        let mut sel = if *upload_port_custom || idx_in_list.is_none() {
            custom_idx
        } else {
            idx_in_list.unwrap_or(custom_idx)
        };

        let selected_label = if sel < n {
            serial_ports[sel].as_str()
        } else if upload_port.is_empty() {
            "— Port —"
        } else {
            "Custom path…"
        };

        theme::combo_box("uart_hw_serial_port")
            .selected_text(RichText::new(selected_label).monospace().size(10.5))
            .width(200.0)
            .show_ui(ui, |ui| {
                for (i, p) in serial_ports.iter().enumerate() {
                    ui.selectable_value(&mut sel, i, RichText::new(p).monospace().size(10.5));
                }
                ui.selectable_value(
                    &mut sel,
                    custom_idx,
                    RichText::new("Custom path…").monospace().size(10.5),
                );
            });

        if sel < n {
            *upload_port = serial_ports[sel].clone();
            *upload_port_custom = false;
        } else {
            *upload_port_custom = true;
        }
    });

    if *upload_port_custom {
        ui.horizontal(|ui| {
            ui.add_space(4.0);
            ui.add(
                TextEdit::singleline(upload_port)
                    .desired_width(220.0)
                    .font(egui::TextStyle::Monospace),
            );
        });
    }

    ui.horizontal(|ui| {
        ui.label(
            RichText::new("Baud")
                .monospace()
                .size(10.5)
                .color(theme::accent_dim()),
        );
        let baud_label = state.hardware_baud.to_string();
        theme::combo_box("uart_hw_baud")
            .selected_text(RichText::new(baud_label).monospace().size(10.5))
            .width(100.0)
            .show_ui(ui, |ui| {
                for &b in HW_BAUD_CHOICES {
                    ui.selectable_value(
                        &mut state.hardware_baud,
                        b,
                        RichText::new(b.to_string()).monospace().size(10.5),
                    );
                }
            });
    });

    ui.add_space(6.0);

    let connected = uart_serial.is_some();
    ui.horizontal(|ui| {
        let can_toggle = !upload_port.trim().is_empty();
        if connected {
            if ui
                .add_enabled(
                    true,
                    Button::new(RichText::new("Disconnect").monospace().size(11.0).color(theme::accent()))
                        .fill(theme::panel_over_wallpaper(ui.ctx(), theme::sim_surface_lift()))
                        .stroke(Stroke::new(1.0, theme::sim_border_bright())),
                )
                .clicked()
            {
                *uart_serial = None;
                uart_serial_error.take();
            }
        } else if ui
            .add_enabled(
                can_toggle,
                Button::new(RichText::new("Connect").monospace().size(11.0).color(theme::accent()))
                    .fill(theme::panel_over_wallpaper(ui.ctx(), theme::sim_surface_lift()))
                    .stroke(Stroke::new(1.0, theme::sim_border_bright())),
            )
            .clicked()
        {
            uart_serial_error.take();
            let path = upload_port.trim();
            match real_uart::open(path, state.hardware_baud) {
                Ok(p) => {
                    *uart_serial = Some(p);
                }
                Err(e) => {
                    *uart_serial_error = Some(e);
                }
            }
        }
    });

    if let Some(ref err) = uart_serial_error {
        ui.label(
            RichText::new(format!("Error: {err}"))
                .monospace()
                .size(10.0)
                .color(theme::err_red()),
        );
    }

    ui.add_space(8.0);
    ui.horizontal(|ui| {
        ui.label(
            RichText::new("Device TX → monitor")
                .monospace()
                .size(11.0)
                .color(theme::accent_dim()),
        );
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if ui
                .small_button(
                    RichText::new("Clear monitor")
                        .monospace()
                        .size(10.0)
                        .color(theme::accent_dim()),
                )
                .clicked()
            {
                state.clear_monitor();
            }
        });
    });
    ui.add_space(2.0);
    terminal_scroll(
        ui,
        "uart_hw_term0",
        state.rx0_shown(),
        USART0_TERM_W,
        USART0_TERM_H,
    );

    ui.add_space(6.0);
    uart_send_row_usb(
        ui,
        "Host → device RX",
        &mut state.line0,
        state.send_line_ending,
        uart_serial,
        uart_serial_error,
    );
}

fn uart_send_row_usb(
    ui:                &mut Ui,
    label:             &str,
    line:              &mut String,
    line_ending:       UartSendLineEnding,
    uart_serial:       &mut Option<Box<dyn SerialPort>>,
    uart_serial_error: &mut Option<String>,
) {
    ui.label(
        RichText::new(label)
            .monospace()
            .size(10.5)
            .color(theme::accent_dim()),
    );
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        let te_width = ((ui.available_width() - 78.0).max(40.0)).min(HOST_RX_SEND_MAX_W);
        let enabled = uart_serial.is_some();
        Frame::NONE
            .fill(theme::panel_over_wallpaper(ui.ctx(), theme::sim_surface_lift()))
            .stroke(Stroke::new(1.0, theme::sim_border_bright()))
            .inner_margin(Margin::symmetric(6, 4))
            .corner_radius(CornerRadius::same(4))
            .show(ui, |ui| {
                ui.set_min_width(te_width);
                ui.set_max_width(te_width);
                ui.add_enabled(
                    enabled,
                    TextEdit::singleline(line)
                        .desired_width(te_width)
                        .font(egui::TextStyle::Monospace)
                        .frame(false),
                );
            });
        if ui
            .add_enabled(
                enabled,
                Button::new(RichText::new("Send").monospace().size(11.0).color(theme::accent()))
                    .fill(theme::panel_over_wallpaper(ui.ctx(), theme::sim_surface_lift()))
                    .stroke(Stroke::new(1.0, theme::sim_border_bright()))
                    .corner_radius(CornerRadius::same(5)),
            )
            .clicked()
        {
            uart_serial_error.take();
            if let Some(port) = uart_serial.as_mut() {
                let r: Result<(), std::io::Error> = (|| {
                    port.write_all(line.as_bytes())?;
                    port.write_all(line_ending.trailing_bytes())?;
                    port.flush()
                })();
                if let Err(e) = r {
                    *uart_serial_error = Some(format!("write failed: {e}"));
                }
            }
            line.clear();
        }
    });
}

fn line_ending_combo(ui: &mut Ui, state: &mut UartPanelState) {
    Frame::NONE
        .fill(theme::panel_over_wallpaper(ui.ctx(), theme::sim_surface_lift()))
        .stroke(Stroke::new(1.0, theme::sim_border_bright()))
        .inner_margin(Margin::symmetric(6, 3))
        .corner_radius(CornerRadius::same(4))
        .show(ui, |ui| {
            theme::combo_box("uart_send_line_ending")
                .width(160.0)
                .selected_text(
                    RichText::new(state.send_line_ending.label())
                        .monospace()
                        .size(10.5)
                        .color(theme::accent()),
                )
                .show_ui(ui, |ui| {
                    ui.style_mut().visuals.override_text_color = Some(theme::accent());
                    ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
                    for v in [
                        UartSendLineEnding::None,
                        UartSendLineEnding::Newline,
                        UartSendLineEnding::CarriageReturn,
                        UartSendLineEnding::BothNlAndCr,
                    ] {
                        ui.selectable_value(
                            &mut state.send_line_ending,
                            v,
                            RichText::new(v.label()).monospace().size(10.5),
                        );
                    }
                });
        });
}

fn estimate_wrapped_rows(text: &str, width_px: f32) -> f32 {
    const CHAR_W: f32 = 7.2;
    let w = width_px.max(48.0);
    let mut rows = 0.0f32;
    for line in text.split('\n') {
        let len = line.chars().count() as f32;
        rows += (len * CHAR_W / w).ceil().max(1.0);
    }
    rows.max(1.0)
}

fn terminal_scroll(
    ui: &mut Ui,
    id: &'static str,
    rx: &str,
    width: f32,
    max_outer_height: f32,
) {
    const LINE_H: f32 = 15.0;
    const FRAME_V_PAD: f32 = 16.0; // inner_margin top + bottom inside Frame
    let inner_max = (max_outer_height - FRAME_V_PAD).max(28.0);
    let content_w = (width - 16.0).max(40.0);
    let display = if rx.is_empty() { "(no output yet)" } else { rx };
    let content_h = estimate_wrapped_rows(display, content_w) * LINE_H;
    let inner_viewport_h = content_h.min(inner_max).max(26.0);
    let outer_h = (inner_viewport_h + FRAME_V_PAD)
        .min(max_outer_height)
        .max(TERM_MIN_OUTER_H);

    ui.allocate_ui(egui::vec2(width, outer_h), |ui| {
        ui.set_min_width(width);
        ui.set_max_width(width);
        ui.set_min_height(outer_h);
        ui.set_max_height(outer_h);
        Frame::NONE
            .fill(theme::search_bg())
            .stroke(Stroke::new(0.75, theme::sim_border()))
            .inner_margin(Margin::symmetric(8, 8))
            .corner_radius(CornerRadius::same(4))
            .show(ui, |ui| {
                ui.set_width(width - 16.0);
                ScrollArea::vertical()
                    .id_salt(id)
                    .max_height(inner_viewport_h)
                    .stick_to_bottom(true)
                    .show(ui, |ui| {
                        let avail = ui.available_width();
                        ui.set_width(avail);
                        ui.add(
                            egui::Label::new(
                                RichText::new(display)
                                    .monospace()
                                    .size(12.0)
                                    .line_height(Some(15.0))
                                    .color(theme::accent_dim()),
                            )
                            .wrap(),
                        );
                    });
            });
    });
}

fn uart_send_row(
    ui: &mut Ui,
    cpu: &mut Cpu,
    label: &str,
    line: &mut String,
    port: u8,
    line_ending: UartSendLineEnding,
) {
    ui.label(
        RichText::new(label)
            .monospace()
            .size(10.5)
            .color(theme::accent_dim()),
    );
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        let te_width = ((ui.available_width() - 78.0).max(40.0)).min(HOST_RX_SEND_MAX_W);
        Frame::NONE
            .fill(theme::panel_over_wallpaper(ui.ctx(), theme::sim_surface_lift()))
            .stroke(Stroke::new(1.0, theme::sim_border_bright()))
            .inner_margin(Margin::symmetric(6, 4))
            .corner_radius(CornerRadius::same(4))
            .show(ui, |ui| {
                ui.set_min_width(te_width);
                ui.set_max_width(te_width);
                ui.add(
                    TextEdit::singleline(line)
                        .desired_width(te_width)
                        .font(egui::TextStyle::Monospace)
                        .frame(false),
                );
            });
        if ui
            .add(
                Button::new(RichText::new("Send").monospace().size(11.0).color(theme::accent()))
                    .fill(theme::panel_over_wallpaper(ui.ctx(), theme::sim_surface_lift()))
                    .stroke(Stroke::new(1.0, theme::sim_border_bright()))
                    .corner_radius(CornerRadius::same(5)),
            )
            .clicked()
        {
            for b in line.bytes() {
                cpu.usart_rx_host_push(port, b);
            }
            line_ending.push_after_payload(cpu, port);
            line.clear();
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crlf_does_not_wipe_line_before_commit() {
        let mut scroll = String::new();
        let mut partial = String::new();
        let mut stab = LineStabilizer {
            enabled: false,
            ..Default::default()
        };
        let mut line_feed = 0usize;
        let mut pending = false;
        feed_bytes_into_rx(
            &mut scroll,
            &mut partial,
            &mut stab,
            b"12345\r\n",
            &mut line_feed,
            &mut pending,
        );
        assert_eq!(scroll, "12345\n");
        assert!(partial.is_empty());
        assert!(!pending);
    }

    #[test]
    fn cr_lf_split_across_feeds_keeps_digits() {
        let mut scroll = String::new();
        let mut partial = String::new();
        let mut stab = LineStabilizer {
            enabled: false,
            ..Default::default()
        };
        let mut line_feed = 0usize;
        let mut pending = false;
        feed_bytes_into_rx(
            &mut scroll,
            &mut partial,
            &mut stab,
            b"999\r",
            &mut line_feed,
            &mut pending,
        );
        assert_eq!(scroll, "");
        assert_eq!(partial.as_str(), "999");
        assert!(pending);
        feed_bytes_into_rx(
            &mut scroll,
            &mut partial,
            &mut stab,
            b"\n",
            &mut line_feed,
            &mut pending,
        );
        assert_eq!(scroll, "999\n");
        assert!(partial.is_empty());
        assert!(!pending);
    }
}
