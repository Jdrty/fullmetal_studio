//! avr_sim_panel tabs cpu ports timers sram

use eframe::egui::{
    self, Button, Color32, Frame, Grid, Margin, RichText, ScrollArea, Stroke, Ui,
};

use crate::avr::cpu::{
    Cpu, StepResult, SREG_C, SREG_H, SREG_I, SREG_N, SREG_S, SREG_T, SREG_V, SREG_Z,
    FLASH_WORDS,
};
use crate::avr::io_map;
use crate::welcome::{START_GREEN, START_GREEN_DIM};

const AMBER:   Color32 = Color32::from_rgb(255, 185, 55);
const DIM:     Color32 = Color32::from_rgb(65,  65,  65);
const ERR_RED: Color32 = Color32::from_rgb(255, 80,  80);

// public_types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SimTab { Cpu, Ports, Timers, Sram }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SimAction {
    None,
    Assemble,
    Step,
    Run10,
    Run100,
    Reset,
    AutoToggle,
}

// entry_point

pub fn show_sim_panel(
    ui:           &mut Ui,
    cpu:          &Cpu,
    last_result:  Option<StepResult>,
    active_tab:   &mut SimTab,
    auto_running: bool,
    ips:          f64,
) -> SimAction {
    let mut action = SimAction::None;

    Frame::NONE
        .fill(Color32::from_rgb(3, 7, 3))
        .stroke(Stroke::new(1.0, START_GREEN_DIM))
        .inner_margin(Margin::same(10))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());

            // sticky_header
            ui.label(
                RichText::new("[ AVR SIM  ATmega128A ]")
                    .monospace().size(13.0).color(START_GREEN),
            );
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(format!("PC  {:04X}", cpu.pc))
                        .monospace().size(12.5).color(AMBER),
                );
                ui.add_space(12.0);
                ui.label(
                    RichText::new(format!("SP  {:04X}", cpu.sp))
                        .monospace().size(12.5).color(START_GREEN_DIM),
                );
                ui.add_space(12.0);
                ui.label(
                    RichText::new(format!("CYC {}", cpu.cycles))
                        .monospace().size(12.5).color(START_GREEN_DIM),
                );
            });
            ui.add_space(6.0);

            // tab_bar
            ui.horizontal(|ui| {
                for (tab, label) in [
                    (SimTab::Cpu,    "CPU"),
                    (SimTab::Ports,  "PORTS"),
                    (SimTab::Timers, "TIMERS"),
                    (SimTab::Sram,   "SRAM"),
                ] {
                    let selected   = *active_tab == tab;
                    let color      = if selected { START_GREEN } else { DIM };
                    let fill       = if selected { Color32::from_rgb(8, 24, 8) }
                                     else        { Color32::from_rgb(3, 7, 3) };
                    let stroke_col = if selected { START_GREEN } else { DIM };
                    let resp = ui.add(
                        Button::new(
                            RichText::new(label).monospace().size(11.5).color(color),
                        )
                        .fill(fill)
                        .stroke(Stroke::new(1.0, stroke_col)),
                    );
                    if resp.clicked() { *active_tab = tab; }
                }
            });
            ui.separator();
            ui.add_space(4.0);

            // scrollable_tab_content
            let avail_h = ui.available_height() - 88.0; // room_for_controls
            ScrollArea::vertical()
                .id_salt("sim_scroll")
                .auto_shrink([false, false])
                .max_height(avail_h.max(120.0))
                .show(ui, |ui| {
                    match *active_tab {
                        SimTab::Cpu    => show_cpu_tab(ui, cpu, last_result),
                        SimTab::Ports  => show_ports_tab(ui, cpu),
                        SimTab::Timers => show_timers_tab(ui, cpu),
                        SimTab::Sram   => show_sram_tab(ui, cpu),
                    }
                });

            // sticky_controls
            ui.add_space(4.0);
            ui.separator();
            ui.add_space(4.0);

            if assemble_btn(ui, "ASSEMBLE  (from editor)").clicked() {
                action = SimAction::Assemble;
            }
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                if retro_btn(ui, "STEP").clicked()        { action = SimAction::Step; }
                if retro_btn(ui, "RUN\u{00D7}10").clicked()  { action = SimAction::Run10; }
                if retro_btn(ui, "RUN\u{00D7}100").clicked() { action = SimAction::Run100; }
                if retro_btn(ui, "RESET").clicked()       { action = SimAction::Reset; }
            });
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                if auto_running {
                    if ui.add(
                        Button::new(
                            RichText::new("\u{25A0} STOP").monospace().size(12.5).color(AMBER),
                        )
                        .fill(Color32::from_rgb(30, 12, 0))
                        .stroke(Stroke::new(1.5, AMBER)),
                    ).clicked() {
                        action = SimAction::AutoToggle;
                    }
                } else if ui.add(
                    Button::new(
                        RichText::new("\u{25B6} AUTO").monospace().size(12.5).color(START_GREEN),
                    )
                    .fill(Color32::from_rgb(6, 20, 6))
                    .stroke(Stroke::new(1.5, START_GREEN)),
                ).clicked() {
                    action = SimAction::AutoToggle;
                }
                ui.add_space(8.0);
                ui.label(
                    RichText::new(fmt_ips(ips, auto_running))
                        .monospace()
                        .size(12.0)
                        .color(if auto_running { AMBER } else { DIM }),
                );
            });
        });

    action
}

// cpu_tab

fn show_cpu_tab(ui: &mut Ui, cpu: &Cpu, last_result: Option<StepResult>) {
    section_label(ui, "REGISTERS");
    ui.add_space(4.0);
    Grid::new("sim_regs")
        .num_columns(4)
        .spacing([10.0, 2.0])
        .show(ui, |ui| {
            for row in 0..8usize {
                for col in 0..4usize {
                    let idx = col * 8 + row;
                    let val = cpu.regs[idx];
                    let color = if val != 0 { START_GREEN } else { DIM };
                    ui.label(
                        RichText::new(format!("R{idx:02}:{val:02X}"))
                            .monospace().size(12.0).color(color),
                    );
                }
                ui.end_row();
            }
        });

    ui.add_space(6.0);
    ui.separator();
    ui.add_space(4.0);

    section_label(ui, "SREG");
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        for &(name, bit) in &[
            ("I", SREG_I), ("T", SREG_T), ("H", SREG_H), ("S", SREG_S),
            ("V", SREG_V), ("N", SREG_N), ("Z", SREG_Z), ("C", SREG_C),
        ] {
            let set = (cpu.sreg >> bit) & 1 != 0;
            let color = if set { AMBER } else { DIM };
            ui.label(
                RichText::new(format!("{name}:{}", (cpu.sreg >> bit) & 1))
                    .monospace().size(12.5).color(color),
            );
        }
    });

    ui.add_space(6.0);
    ui.separator();
    ui.add_space(4.0);

    section_label(ui, "FLASH DISASM");
    ui.add_space(4.0);
    let pc    = cpu.pc;
    let start = pc.saturating_sub(3);
    for addr in start..start + 8 {
        if addr as usize >= FLASH_WORDS { break; }
        let is_current = addr == pc;
        let arrow  = if is_current { "\u{2192}" } else { " " };
        let disasm = cpu.disasm_at(addr);
        let (color, size) = if is_current { (AMBER, 13.0_f32) } else { (START_GREEN_DIM, 12.0_f32) };
        // ivt_name_if_match
        let ivt_ann = Cpu::ivt_name(addr as u32)
            .map(|n| format!("  ; <{n}>"))
            .unwrap_or_default();
        ui.label(
            RichText::new(format!("{arrow} {:04X}  {disasm}{ivt_ann}", addr))
                .monospace().size(size).color(color),
        );
    }

    if let Some(res) = last_result {
        ui.add_space(4.0);
        match res {
            StepResult::UnknownOpcode(op) => {
                ui.label(
                    RichText::new(format!("! UNKNOWN OPCODE 0x{op:04X}"))
                        .monospace().size(11.5).color(ERR_RED),
                );
            }
            StepResult::Halted => {
                ui.label(
                    RichText::new("! HALTED (PC out of Flash)")
                        .monospace().size(11.5).color(AMBER),
                );
            }
            StepResult::Ok => {}
        }
    }
}

// ports_tab

fn show_ports_tab(ui: &mut Ui, cpu: &Cpu) {
    section_label(ui, "GPIO PORTS  (DDR=0 INPUT, DDR=1 OUTPUT)");
    ui.add_space(6.0);

    ui.label(
        RichText::new("PORT  DDR   OUT   PIN   7 6 5 4 3 2 1 0")
            .monospace().size(11.5).color(START_GREEN_DIM),
    );
    ui.add_space(2.0);
    ui.separator();

    for &(name, port_addr, ddr_addr, pin_addr) in &io_map::PORTS {
        let port = cpu.read_mem(port_addr);
        let ddr  = cpu.read_mem(ddr_addr);
        let pin  = cpu.read_mem(pin_addr);

        let mut bits   = String::with_capacity(16);
        let mut colors: Vec<Color32> = Vec::with_capacity(16);
        for bit in (0..8u8).rev() {
            let is_out = (ddr >> bit) & 1 != 0;
            let high   = if is_out { (port >> bit) & 1 != 0 }
                         else      { (pin  >> bit) & 1 != 0 };
            if is_out {
                if high { bits.push('\u{2588}'); colors.push(AMBER); }
                else    { bits.push('\u{2591}'); colors.push(START_GREEN_DIM); }
            } else {
                bits.push('\u{00B7}'); colors.push(DIM);
            }
            bits.push(' ');
            colors.push(DIM);
        }

        ui.add_space(2.0);
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(format!("{name}     {ddr:02X}    {port:02X}    {pin:02X}    "))
                    .monospace().size(12.0).color(START_GREEN),
            );
            for (i, ch) in bits.chars().enumerate() {
                if ch == ' ' { continue; }
                let bit_idx = i / 2;
                let col = colors[i];
                ui.label(RichText::new(ch.to_string()).monospace().size(13.0).color(col));
                if bit_idx < 7 { ui.add_space(-4.0); }
            }
        });
    }

    ui.add_space(8.0);
    ui.separator();
    ui.add_space(4.0);
    ui.label(
        RichText::new("  \u{2588} OUT HIGH    \u{2591} OUT LOW    \u{00B7} INPUT")
            .monospace().size(11.0).color(START_GREEN_DIM),
    );
}

// timers_tab

fn show_timers_tab(ui: &mut Ui, cpu: &Cpu) {
    // data_addr_to_io_idx
    let io = &cpu.io;
    let ix = |a: u16| -> u8 { io[(a as usize) - 0x0020] };

    let tifr  = ix(io_map::TIFR);
    let timsk = ix(io_map::TIMSK);

    // timer0_ui
    timer_section(ui, "TIMER 0", "(8-bit)");

    let tccr0 = ix(io_map::TCCR0);
    let tcnt0 = ix(io_map::TCNT0);
    let ocr0  = ix(io_map::OCR0);
    let cs0   = tccr0 & 0x07;
    let ctc0  = (tccr0 & 0x08) != 0;

    Grid::new("t0_grid").num_columns(3).spacing([8.0, 2.0]).show(ui, |ui| {
        kv3(ui, "TCCR0", &format!("{tccr0:02X}"),
            &format!("{}  {}", t01_cs_str(cs0), if ctc0 { "CTC" } else { "Normal" }));
        kv3(ui, "TCNT0", &format!("{tcnt0:02X}"), &format!("[{}]", tcnt0));
        kv3(ui, "OCR0",  &format!("{ocr0:02X}"),  &format!("[{}]", ocr0));
    });
    ui.add_space(2.0);
    ui.horizontal(|ui| {
        flag_lbl(ui, "TOV",  tifr & 0x01 != 0);
        flag_lbl(ui, "OCF",  tifr & 0x02 != 0);
        ui.label(RichText::new(" | ").monospace().size(11.0).color(DIM));
        flag_lbl(ui, "TOIE", timsk & 0x01 != 0);
        flag_lbl(ui, "OCIE", timsk & 0x02 != 0);
    });

    ui.add_space(6.0);
    ui.separator();
    ui.add_space(4.0);

    // timer1_ui
    timer_section(ui, "TIMER 1", "(16-bit)");

    let tccr1a = ix(io_map::TCCR1A);
    let tccr1b = ix(io_map::TCCR1B);
    let tcnt1  = (ix(io_map::TCNT1H) as u16) << 8 | ix(io_map::TCNT1L) as u16;
    let ocr1a  = (ix(io_map::OCR1AH) as u16) << 8 | ix(io_map::OCR1AL) as u16;
    let ocr1b  = (ix(io_map::OCR1BH) as u16) << 8 | ix(io_map::OCR1BL) as u16;
    let cs1    = tccr1b & 0x07;
    let ctc1   = (tccr1b & 0x08) != 0;

    Grid::new("t1_grid").num_columns(3).spacing([8.0, 2.0]).show(ui, |ui| {
        kv3(ui, "TCCR1A", &format!("{tccr1a:02X}"), "");
        kv3(ui, "TCCR1B", &format!("{tccr1b:02X}"),
            &format!("{}  {}", t01_cs_str(cs1), if ctc1 { "CTC" } else { "Normal" }));
        kv3(ui, "TCNT1",  &format!("{tcnt1:04X}"), &format!("[{}]", tcnt1));
        kv3(ui, "OCR1A",  &format!("{ocr1a:04X}"), &format!("[{}]", ocr1a));
        kv3(ui, "OCR1B",  &format!("{ocr1b:04X}"), &format!("[{}]", ocr1b));
    });
    ui.add_space(2.0);
    ui.horizontal(|ui| {
        flag_lbl(ui, "TOV1",   tifr & 0x04 != 0);
        flag_lbl(ui, "OCF1A",  tifr & 0x10 != 0);
        flag_lbl(ui, "OCF1B",  tifr & 0x08 != 0);
        ui.label(RichText::new(" | ").monospace().size(11.0).color(DIM));
        flag_lbl(ui, "TOIE1",  timsk & 0x04 != 0);
        flag_lbl(ui, "OCIE1A", timsk & 0x10 != 0);
        flag_lbl(ui, "OCIE1B", timsk & 0x08 != 0);
    });

    ui.add_space(6.0);
    ui.separator();
    ui.add_space(4.0);

    // timer2_ui
    timer_section(ui, "TIMER 2", "(8-bit async)");

    let tccr2 = ix(io_map::TCCR2);
    let tcnt2 = ix(io_map::TCNT2);
    let ocr2  = ix(io_map::OCR2);
    let cs2   = tccr2 & 0x07;
    let ctc2  = (tccr2 & 0x08) != 0;

    Grid::new("t2_grid").num_columns(3).spacing([8.0, 2.0]).show(ui, |ui| {
        kv3(ui, "TCCR2", &format!("{tccr2:02X}"),
            &format!("{}  {}", t2_cs_str(cs2), if ctc2 { "CTC" } else { "Normal" }));
        kv3(ui, "TCNT2", &format!("{tcnt2:02X}"), &format!("[{}]", tcnt2));
        kv3(ui, "OCR2",  &format!("{ocr2:02X}"),  &format!("[{}]", ocr2));
    });
    ui.add_space(2.0);
    ui.horizontal(|ui| {
        flag_lbl(ui, "TOV2",  tifr & 0x40 != 0);
        flag_lbl(ui, "OCF2",  tifr & 0x80 != 0);
        ui.label(RichText::new(" | ").monospace().size(11.0).color(DIM));
        flag_lbl(ui, "TOIE2", timsk & 0x40 != 0);
        flag_lbl(ui, "OCIE2", timsk & 0x80 != 0);
    });

    ui.add_space(6.0);
    ui.separator();
    ui.add_space(4.0);

    // timsk_tifr_raw
    section_label(ui, "REGISTERS (raw)");
    ui.add_space(2.0);
    Grid::new("tmr_raw").num_columns(3).spacing([8.0, 2.0]).show(ui, |ui| {
        kv3(ui, "TIMSK", &format!("{timsk:02X}"), &format!("{timsk:08b}b"));
        kv3(ui, "TIFR",  &format!("{tifr:02X}"),  &format!("{tifr:08b}b"));
    });
}

// sram_tab
fn show_sram_tab(ui: &mut Ui, cpu: &Cpu) {
    let sp = cpu.sp;

    // sp_status
    section_label(ui, "SRAM  0x0100 – 0x10FF  (4 096 bytes)");
    ui.add_space(4.0);

    ui.horizontal(|ui| {
        ui.label(RichText::new("SP →").monospace().size(12.0).color(AMBER));
        ui.label(
            RichText::new(format!("0x{sp:04X}"))
                .monospace().size(12.0).color(START_GREEN),
        );
        let sp_in_sram = sp >= 0x0100 && sp <= 0x10FF;
        if sp_in_sram {
            let depth = 0x10FF_u16.wrapping_sub(sp);
            ui.add_space(8.0);
            ui.label(
                RichText::new(format!("(stack depth: {depth} B)"))
                    .monospace().size(11.0).color(START_GREEN_DIM),
            );
        } else if sp == 0x0000 {
            ui.add_space(8.0);
            ui.label(
                RichText::new("(uninitialized)").monospace().size(11.0).color(DIM),
            );
        }
    });
    ui.add_space(4.0);

    // sp_row_index
    let sp_row: Option<usize> = if sp >= 0x0100 && sp <= 0x10FF {
        let sram_off = (sp - 0x0100) as usize;
        Some(sram_off / 8)
    } else {
        None
    };

    let sram  = &cpu.sram;
    let rows  = sram.len() / 8; // 512x8

    Grid::new("sram_grid")
        .num_columns(10)  // addr 8bytes mark
        .spacing([5.0, 1.5])
        .show(ui, |ui| {
            // header
            ui.label(RichText::new("ADDR").monospace().size(11.0).color(START_GREEN_DIM));
            for col in 0..8usize {
                ui.label(
                    RichText::new(format!("+{col:X}"))
                        .monospace().size(11.0).color(START_GREEN_DIM),
                );
            }
            ui.label(RichText::new("").monospace().size(11.0).color(DIM)); // mark_hdr
            ui.end_row();

            // data_rows
            let mut skipping = false;

            for row in 0..rows {
                let base       = row * 8;
                let addr       = 0x0100u32 + base as u32;
                let slice      = &sram[base..base + 8];
                let all0       = slice.iter().all(|&b| b == 0);
                let is_sp_row  = sp_row == Some(row);

                // show_sp_row row0 nonzero_rows
                if all0 && row > 0 && !is_sp_row {
                    if !skipping {
                        skipping = true;
                        ui.label(RichText::new("  ···").monospace().size(10.5).color(DIM));
                        for _ in 0..8 {
                            ui.label(RichText::new("··").monospace().size(10.5).color(DIM));
                        }
                        ui.label(RichText::new("").monospace().size(10.5).color(DIM));
                        ui.end_row();
                    }
                    continue;
                }
                skipping = false;

                // addr_col
                let addr_color = if is_sp_row { AMBER } else { START_GREEN_DIM };
                ui.label(
                    RichText::new(format!("{addr:04X}"))
                        .monospace().size(11.0).color(addr_color),
                );

                // byte_cols
                for (col_idx, &b) in slice.iter().enumerate() {
                    let byte_addr = addr + col_idx as u32;
                    let is_sp_byte = byte_addr == sp as u32;
                    let color = if is_sp_byte { AMBER }
                                else if b != 0 { START_GREEN }
                                else { DIM };
                    ui.label(
                        RichText::new(format!("{b:02X}"))
                            .monospace().size(11.0).color(color),
                    );
                }

                // sp_marker_col
                if is_sp_row {
                    ui.label(
                        RichText::new(format!("\u{2190} SP {:04X}", sp))
                            .monospace().size(10.5).color(AMBER),
                    );
                } else {
                    ui.label(RichText::new("").monospace().size(11.0).color(DIM));
                }
                ui.end_row();
            }
        });
}

// widget_formatting_helpers
fn section_label(ui: &mut Ui, text: &str) {
    ui.label(RichText::new(text).monospace().size(11.0).color(START_GREEN_DIM));
}

fn timer_section(ui: &mut Ui, name: &str, detail: &str) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(name).monospace().size(12.0).color(START_GREEN));
        ui.add_space(4.0);
        ui.label(RichText::new(detail).monospace().size(11.0).color(START_GREEN_DIM));
    });
    ui.add_space(2.0);
}

/// kv_row grid helper
fn kv3(ui: &mut Ui, key: &str, val: &str, ann: &str) {
    ui.label(RichText::new(key).monospace().size(11.0).color(START_GREEN_DIM));
    let vcolor = if val.trim_start_matches('0').is_empty() || val == "0000" || val == "00" {
        DIM
    } else {
        AMBER
    };
    ui.label(RichText::new(val).monospace().size(11.0).color(vcolor));
    ui.label(RichText::new(ann).monospace().size(11.0).color(DIM));
    ui.end_row();
}

fn flag_lbl(ui: &mut Ui, name: &str, set: bool) {
    let color = if set { AMBER } else { DIM };
    ui.label(
        RichText::new(format!("{name}:{}", u8::from(set)))
            .monospace().size(11.0).color(color),
    );
}

fn t01_cs_str(cs: u8) -> &'static str {
    match cs {
        0 => "stopped", 1 => "CLK/1",  2 => "CLK/8",
        3 => "CLK/64",  4 => "CLK/256", 5 => "CLK/1024",
        6 => "ext↓",    7 => "ext↑",    _ => "?",
    }
}

fn t2_cs_str(cs: u8) -> &'static str {
    match cs {
        0 => "stopped",  1 => "CLK/1",   2 => "CLK/8",
        3 => "CLK/32",   4 => "CLK/64",  5 => "CLK/128",
        6 => "CLK/256",  7 => "CLK/1024", _ => "?",
    }
}

fn retro_btn(ui: &mut Ui, label: &str) -> egui::Response {
    ui.add(
        Button::new(RichText::new(label).monospace().size(12.0).color(START_GREEN))
            .fill(Color32::from_rgb(6, 16, 6))
            .stroke(Stroke::new(1.0, START_GREEN_DIM)),
    )
}

fn assemble_btn(ui: &mut Ui, label: &str) -> egui::Response {
    ui.add(
        Button::new(RichText::new(label).monospace().size(12.5).color(START_GREEN))
            .fill(Color32::from_rgb(8, 24, 8))
            .stroke(Stroke::new(1.0, START_GREEN)),
    )
}

fn fmt_ips(ips: f64, running: bool) -> String {
    if !running && ips == 0.0 { return "---".into(); }
    if ips >= 1_000_000.0 {
        format!("{:.2} MIPS", ips / 1_000_000.0)
    } else if ips >= 1_000.0 {
        format!("{:.1} kIPS", ips / 1_000.0)
    } else {
        format!("{:.0} IPS", ips)
    }
}
