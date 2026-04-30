//! cost_analysis rhs_panel simulate attribute_cycles per_label heuristics

use std::collections::{HashMap, HashSet, VecDeque};

use eframe::egui::{
    self, Button, Color32, CursorIcon, Frame, Label, Margin, RichText, ScrollArea, Sense, Stroke,
    Ui,
};

use crate::avr::assembler::assemble_full;
use crate::avr::cpu::{Cpu, StepResult};
use crate::avr::io_map;
use crate::avr::parse_board_from_source;
use crate::avr::McuModel;
use crate::theme;

const MAX_SIM_STEPS: u64 = 3_000_000;

pub struct CostHelperState {
    pub file_idx: usize,
    prev_file_idx: usize,
    cache: Option<(usize, Result<AnalysisReport, String>)>,
    pub disabled_labels: HashSet<String>,
}

impl Default for CostHelperState {
    fn default() -> Self {
        Self {
            file_idx:        0,
            prev_file_idx:   usize::MAX,
            cache:           None,
            disabled_labels: HashSet::new(),
        }
    }
}

#[derive(Clone)]
struct LabelCostRow {
    name:            String,
    addr:            u32,
    cycles_compute:  u64,
    cycles_io_wait:  u64,
    static_m:        u64,
    static_x:        u64,
    nop_cnt:         u32,
    push_cnt:        u32,
    pop_cnt:         u32,
}

#[derive(Clone)]
struct AnalysisReport {
    board:            McuModel,
    sim_steps:        u64,
    sim_cycles:       u64,
    sim_compute:      u64,
    sim_io_wait:      u64,
    outcome:          StepResult,
    hit_step_cap:     bool,
    rows:             Vec<LabelCostRow>,
    hints:            Vec<String>,
}

fn cycles_in_range(flash: &[u16], addr_a: u32, addr_b: u32) -> (u64, u64) {
    let (start, end) = if addr_a <= addr_b {
        (addr_a as usize, addr_b as usize)
    } else {
        (addr_b as usize, addr_a as usize)
    };
    let mut min_total: u64 = 0;
    let mut max_total: u64 = 0;
    let mut i = start;
    while i < end && i < flash.len() {
        let op = flash[i];
        let (mn, mx) = Cpu::instr_cycles(op);
        min_total += mn as u64;
        max_total += mx as u64;
        i += Cpu::instr_words(op);
    }
    (min_total, max_total)
}

fn count_nop_push_pop(flash: &[u16], start: u32, end: u32) -> (u32, u32, u32) {
    let (lo, hi) = (start as usize, end as usize);
    let mut nops = 0u32;
    let mut push = 0u32;
    let mut pop = 0u32;
    let mut i = lo.min(flash.len());
    let hi = hi.min(flash.len());
    while i < hi {
        let op = flash[i];
        if op == 0 {
            nops += 1;
        }
        if op & 0xFE0F == 0x920F {
            push += 1;
        }
        if op & 0xFE0F == 0x900F {
            pop += 1;
        }
        i += Cpu::instr_words(op);
    }
    (nops, push, pop)
}

fn code_label_regions(labels: &HashMap<String, u32>, flash_len: usize) -> Vec<(String, u32)> {
    let mut pairs: Vec<(u32, String)> = labels
        .iter()
        .filter(|(_, &a)| (a as usize) < flash_len)
        .map(|(n, &a)| (a, n.clone()))
        .collect();
    pairs.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));

    let mut out: Vec<(String, u32)> = Vec::new();
    for (addr, name) in pairs {
        if let Some((ref mut nm, prev_a)) = out.last_mut() {
            if *prev_a == addr {
                nm.push_str(" / ");
                nm.push_str(&name);
                continue;
            }
        }
        out.push((name, addr));
    }
    out.sort_by_key(|(_, a)| *a);
    out
}

fn all_labels_in_flash_order(labels: &HashMap<String, u32>, flash_len: usize) -> Vec<(String, u32)> {
    let mut v: Vec<(String, u32)> = labels
        .iter()
        .filter(|(_, &a)| (a as usize) < flash_len)
        .map(|(n, &a)| (n.clone(), a))
        .collect();
    v.sort_by(|a, b| a.1.cmp(&b.1).then_with(|| a.0.cmp(&b.0)));
    v
}

fn merged_key_for_addr(addr: u32, merged_regions: &[(String, u32)]) -> Option<&str> {
    merged_regions
        .iter()
        .find(|(_, a)| *a == addr)
        .map(|(n, _)| n.as_str())
}

fn region_name_for_pc(pc: u32, regions: &[(String, u32)]) -> String {
    if regions.is_empty() {
        return "(program)".to_string();
    }
    if pc < regions[0].1 {
        return format!("(before `{}`)", regions[0].0);
    }
    let mut lo = 0usize;
    let mut hi = regions.len();
    while lo < hi {
        let mid = (lo + hi) / 2;
        if regions[mid].1 <= pc {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
    regions[lo.saturating_sub(1)].0.clone()
}

fn static_span_end(flash_words: u32, regions: &[(String, u32)], start: u32) -> u32 {
    let i = regions.partition_point(|(_, a)| *a < start);
    if i < regions.len() && regions[i].1 == start {
        return regions
            .get(i + 1)
            .map(|(_, a)| *a)
            .unwrap_or(flash_words);
    }
    if i < regions.len() {
        return regions[i].1;
    }
    flash_words
}

fn icr1l_mem(model: McuModel) -> u16 {
    match model {
        McuModel::Atmega328P => 0x0086,
        McuModel::Atmega128A => io_map::ICR1L,
    }
}

fn io_byte_idx(data_addr: u16) -> usize {
    (data_addr.saturating_sub(0x0020)) as usize
}

fn peek_lds_k(flash: &[u16], pc: u32) -> Option<u16> {
    let pc = pc as usize;
    if pc + 1 >= flash.len() {
        return None;
    }
    let op = flash[pc];
    let nibble_hi = (op >> 8) & 0xF;
    if !matches!(nibble_hi, 0 | 1) {
        return None;
    }
    if (op & 0x0F) != 0 {
        return None;
    }
    Some(flash[pc + 1])
}

fn peek_in_data_addr(flash: &[u16], pc: u32) -> Option<u16> {
    let op = *flash.get(pc as usize)?;
    if (op >> 12) != 0xB || (op & 0x0800) != 0 {
        return None;
    }
    let a = ((op >> 5) & 0x30) | (op & 0x0F);
    Some(io_map::io_to_mem(a as u8))
}

fn peek_sbic_sbis_data_addr(op: u16) -> Option<u16> {
    match op & 0xFF00 {
        0x9900 | 0x9B00 => {
            let io_off = ((op >> 3) & 0x1F) as u8;
            Some(io_map::io_to_mem(io_off))
        }
        _ => None,
    }
}

fn data_addr_is_poll_status(model: McuModel, mem: u16) -> bool {
    match model {
        McuModel::Atmega328P => matches!(
            mem,
            io_map::TIFR0_328P
                | io_map::TIFR1_328P
                | io_map::TIFR2_328P
                | io_map::UCSR0A_328P
                | io_map::UCSR0B_328P
                | io_map::UDR0_328P
                | 0x004D // SPSR (I/O 0x2D) — SPI status poll
                | 0x007A
                | 0x007B
                | 0x007C
        ),
        McuModel::Atmega128A => matches!(
            mem,
            io_map::TIFR
                | io_map::UCSR0A
                | io_map::UCSR0B
                | io_map::UDR0
                | io_map::UCSR1A
                | io_map::UCSR1B
                | io_map::UDR1
                | io_map::ADCSRA
                | io_map::ADMUX
                | io_map::SPSR
        ),
    }
}

fn insn_is_status_poll(flash: &[u16], pc: u32, model: McuModel) -> bool {
    let op = match flash.get(pc as usize) {
        Some(&x) => x,
        None => return false,
    };
    if let Some(mem) = peek_in_data_addr(flash, pc) {
        if data_addr_is_poll_status(model, mem) {
            return true;
        }
    }
    if let Some(k) = peek_lds_k(flash, pc) {
        if data_addr_is_poll_status(model, k) {
            return true;
        }
    }
    if let Some(mem) = peek_sbic_sbis_data_addr(op) {
        if data_addr_is_poll_status(model, mem) {
            return true;
        }
    }
    false
}

const SPIN_PC_WINDOW: usize = 28;
const SPIN_MAX_UNIQ: usize = 9;
const SPIN_MIN_REPEAT: usize = 5;

fn detect_io_wait_spin(pc_ring: &VecDeque<u32>, flash: &[u16], model: McuModel) -> bool {
    if pc_ring.len() < 12 {
        return false;
    }
    let slice: Vec<u32> = pc_ring.iter().rev().take(SPIN_PC_WINDOW).copied().collect();
    let uniq: HashSet<u32> = slice.iter().copied().collect();
    if uniq.len() > SPIN_MAX_UNIQ {
        return false;
    }
    let mut freq: HashMap<u32, usize> = HashMap::new();
    for p in &slice {
        *freq.entry(*p).or_insert(0) += 1;
    }
    let max_rep = freq.values().copied().max().unwrap_or(0);
    if max_rep < SPIN_MIN_REPEAT {
        return false;
    }
    uniq.iter().any(|&p| insn_is_status_poll(flash, p, model))
}

struct RegionCycles {
    compute: u64,
    io_wait: u64,
    min_pc:  u32,
}

const ICF1_MASK: u8 = 1 << 5;

fn stub_cost_analysis_periphery(cpu: &mut Cpu, flash: &[u16], icr_seq: &mut u16) {
    match cpu.model {
        McuModel::Atmega328P => {
            let ti = io_byte_idx(io_map::TIFR1_328P);
            if ti < cpu.io.len() {
                cpu.io[ti] |= ICF1_MASK;
            }
            let ua = io_byte_idx(io_map::UCSR0A_328P);
            if ua < cpu.io.len() {
                cpu.io[ua] |= 0x20; // UDRE0
            }
        }
        McuModel::Atmega128A => {
            let ti = io_byte_idx(io_map::TIFR);
            if ti < cpu.io.len() {
                cpu.io[ti] |= ICF1_MASK;
            }
            let ua = io_byte_idx(io_map::UCSR0A);
            if ua < cpu.io.len() {
                cpu.io[ua] |= 0x20;
            }
        }
    }

    let icr_lo = icr1l_mem(cpu.model);
    if peek_lds_k(flash, cpu.pc) == Some(icr_lo) {
        let v = *icr_seq;
        *icr_seq = icr_seq.wrapping_add(0x0157);
        let il = io_byte_idx(icr_lo);
        let ih = io_byte_idx(icr_lo.wrapping_add(1));
        if il < cpu.io.len() {
            cpu.io[il] = v as u8;
        }
        if ih < cpu.io.len() {
            cpu.io[ih] = (v >> 8) as u8;
        }
    }
}

fn run_analysis(source: &str) -> Result<AnalysisReport, String> {
    let board = parse_board_from_source(source)
        .map_err(|e| e.iter().map(|x| x.to_string()).collect::<Vec<_>>().join("; "))?;
    let (flash, _, labels) = assemble_full(source).map_err(|e| {
        e.iter().map(|x| x.to_string()).collect::<Vec<_>>().join("; ")
    })?;
    let flash_len = flash.len();
    if flash_len == 0 {
        return Err("empty flash after assembly".to_string());
    }

    let merged_regions = code_label_regions(&labels, flash_len);

    let mut cpu = Cpu::new_for_model(board);
    cpu.load_flash(&flash);
    cpu.reset();

    let mut by_region: HashMap<String, RegionCycles> = HashMap::new();
    let mut pc_ring: VecDeque<u32> = VecDeque::with_capacity(40);
    let mut steps = 0u64;
    let mut outcome = StepResult::Ok;
    let mut hit_cap = false;

    let mut icr_seq = 0x0200u16;

    loop {
        if cpu.pc as usize >= flash_len {
            outcome = StepResult::Halted;
            break;
        }
        if steps >= MAX_SIM_STEPS {
            hit_cap = true;
            break;
        }

        stub_cost_analysis_periphery(&mut cpu, &flash, &mut icr_seq);

        let pc = cpu.pc;
        pc_ring.push_back(pc);
        while pc_ring.len() > 40 {
            pc_ring.pop_front();
        }
        let io_bound = detect_io_wait_spin(&pc_ring, &flash, board);

        let c0 = cpu.cycles;
        let r = cpu.step();
        let dc = cpu.cycles.saturating_sub(c0);
        let name = region_name_for_pc(pc, &merged_regions);
        by_region
            .entry(name)
            .and_modify(|e| {
                if io_bound {
                    e.io_wait += dc;
                } else {
                    e.compute += dc;
                }
                e.min_pc = e.min_pc.min(pc);
            })
            .or_insert_with(|| RegionCycles {
                compute: if io_bound { 0 } else { dc },
                io_wait: if io_bound { dc } else { 0 },
                min_pc:  pc,
            });
        steps += 1;

        if r != StepResult::Ok {
            outcome = r;
            break;
        }
    }

    let sim_cycles = cpu.cycles;
    let (sim_compute, sim_io_wait) =
        by_region
            .values()
            .fold((0u64, 0u64), |(a, b), e| (a + e.compute, b + e.io_wait));
    let flash_u32 = flash_len as u32;

    let mut rows: Vec<LabelCostRow> = Vec::new();

    for (name, addr) in all_labels_in_flash_order(&labels, flash_len) {
        let (cyc_c, cyc_i) = merged_key_for_addr(addr, &merged_regions)
            .and_then(|k| by_region.get(k))
            .map(|e| (e.compute, e.io_wait))
            .unwrap_or((0, 0));

        let region_end = static_span_end(flash_u32, &merged_regions, addr)
            .min(flash_u32)
            .max(addr.saturating_add(1));

        let (smn, smx) = cycles_in_range(&flash, addr, region_end);
        let (nops, push, pop) = count_nop_push_pop(&flash, addr, region_end);

        rows.push(LabelCostRow {
            name:           name.clone(),
            addr,
            cycles_compute: cyc_c,
            cycles_io_wait: cyc_i,
            static_m:       smn,
            static_x:       smx,
            nop_cnt:        nops,
            push_cnt:       push,
            pop_cnt:        pop,
        });
    }

    // Synthetic regions only the simulator creates (e.g. vector gap before first label).
    for (name, agg) in &by_region {
        if name.starts_with('(') && merged_key_for_addr(agg.min_pc, &merged_regions).is_none() {
            let addr = agg.min_pc;
            if rows.iter().any(|r| r.name == *name) {
                continue;
            }
            let region_end = static_span_end(flash_u32, &merged_regions, addr)
                .min(flash_u32)
                .max(addr.saturating_add(1));
            let (smn, smx) = cycles_in_range(&flash, addr, region_end);
            let (nops, push, pop) = count_nop_push_pop(&flash, addr, region_end);
            rows.push(LabelCostRow {
                name:           name.clone(),
                addr,
                cycles_compute: agg.compute,
                cycles_io_wait: agg.io_wait,
                static_m:       smn,
                static_x:       smx,
                nop_cnt:        nops,
                push_cnt:       push,
                pop_cnt:        pop,
            });
        }
    }

    rows.sort_by(|a, b| a.addr.cmp(&b.addr).then_with(|| a.name.cmp(&b.name)));

    let mut hints: Vec<String> = Vec::new();
    let mut labels_per_addr: HashMap<u32, usize> = HashMap::new();
    for (_, a) in all_labels_in_flash_order(&labels, flash_len) {
        *labels_per_addr.entry(a).or_insert(0) += 1;
    }
    if labels_per_addr.values().any(|&c| c > 1) {
        hints.push(
            "Some flash words have multiple label names, dynamic cycles are repeated on each name; do not sum the % column across those rows."
                .to_string(),
        );
    }
    if let StepResult::UnknownOpcode(op) = outcome {
        hints.push(format!("Unknown opcode 0x{op:04X} — simulation stopped early."));
    }
    hints.push(
        "I/O-wait vs compute: small loops polling timer/UART/ADC/SPI status vs everything else; panel stubs ICF1/UDRE so runs can finish."
            .to_string(),
    );

    for row in &rows {
        let row_cycles = row.cycles_compute + row.cycles_io_wait;
        if row_cycles == 0 {
            continue;
        }
        if row.nop_cnt >= 3 && !row.name.starts_with('(') {
            hints.push(format!(
                "`{}`: many NOPs in this span, remove padding or replace with real work.",
                row.name
            ));
        }
        let d = row.push_cnt as i32 - row.pop_cnt as i32;
        if d.abs() >= 2 {
            hints.push(format!(
                "`{}`: PUSH/POP counts may not match on all paths — verify register restore before every RET.",
                row.name
            ));
        }
        if row.push_cnt >= 6 {
            hints.push(format!(
                "`{}`: heavy callee-saved register use, try reusing registers or shrinking the clobber set.",
                row.name
            ));
        }
    }

    // Duplicate / similar hints
    hints.sort();
    hints.dedup();

    Ok(AnalysisReport {
        board,
        sim_steps: steps,
        sim_cycles,
        sim_compute,
        sim_io_wait,
        outcome,
        hit_step_cap: hit_cap,
        rows,
        hints,
    })
}

fn outcome_label(r: StepResult, hit_step_cap: bool) -> &'static str {
    if hit_step_cap {
        return "stopped (step budget)";
    }
    match r {
        StepResult::Ok => "ok",
        StepResult::Halted => "halted (PC past flash)",
        StepResult::UnknownOpcode(_) => "unknown opcode",
    }
}

fn paint_report(ui: &mut Ui, rep: &AnalysisReport, disabled: &mut HashSet<String>) {
    let valid: HashSet<&str> = rep.rows.iter().map(|r| r.name.as_str()).collect();
    disabled.retain(|n| valid.contains(n.as_str()));

    let d = rep.sim_cycles.max(1) as f32;
    let pc = (rep.sim_compute as f32 / d) * 100.0;
    let pi = (rep.sim_io_wait as f32 / d) * 100.0;
    ui.add(
        Label::new(
            RichText::new(format!(
                "{} · {} sim steps · {} cyc — compute {:.0} ({:.1}%) · I/O-wait {:.0} ({:.1}%) · {}",
                rep.board.label(),
                rep.sim_steps,
                rep.sim_cycles,
                rep.sim_compute,
                pc,
                rep.sim_io_wait,
                pi,
                outcome_label(rep.outcome, rep.hit_step_cap)
            ))
            .monospace()
            .size(10.5)
            .color(theme::start_green_dim()),
        )
        .wrap(),
    );
    ui.add_space(6.0);

    let included_total: u64 = rep
        .rows
        .iter()
        .filter(|r| !disabled.contains(&r.name))
        .map(|r| r.cycles_compute + r.cycles_io_wait)
        .sum();
    let n_included = rep.rows.iter().filter(|r| !disabled.contains(&r.name)).count();
    let denom_inc = included_total.max(1) as f32;

    if n_included == 0 && !rep.rows.is_empty() {
        ui.add(
            Label::new(
                RichText::new("All labels are excluded from % totals — enable at least one for %C / %I.")
                    .monospace()
                    .size(10.5)
                    .color(theme::err_red()),
            )
            .wrap(),
        );
        ui.add_space(4.0);
    }

    if !disabled.is_empty() {
        let inc_c: u64 = rep
            .rows
            .iter()
            .filter(|r| !disabled.contains(&r.name))
            .map(|r| r.cycles_compute)
            .sum();
        let inc_i: u64 = rep
            .rows
            .iter()
            .filter(|r| !disabled.contains(&r.name))
            .map(|r| r.cycles_io_wait)
            .sum();
        let pic = (inc_c as f32 / denom_inc) * 100.0;
        let pii = (inc_i as f32 / denom_inc) * 100.0;
        ui.add(
            Label::new(
                RichText::new(format!(
                    "Included for % columns: {}/{} labels · {} cyc — compute {:.0} ({:.1}%) · I/O-wait {:.0} ({:.1}%)",
                    n_included,
                    rep.rows.len(),
                    included_total,
                    inc_c,
                    pic,
                    inc_i,
                    pii
                ))
                .monospace()
                .size(10.0)
                .color(theme::dim_gray()),
            )
            .wrap(),
        );
        ui.add_space(4.0);
    }

    ui.add(
        Label::new(
            RichText::new(
                "Click a label to exclude/include it from %C / %I (raw cycles unchanged; row grayed when off)",
            )
            .monospace()
            .size(10.5)
            .color(theme::focus()),
        )
        .wrap(),
    );
    ui.add_space(4.0);

    ScrollArea::vertical()
        .id_salt("cost_rows")
        .max_height(280.0)
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ScrollArea::horizontal()
                .id_salt("cost_rows_h")
                .auto_shrink([false, false])
                .show(ui, |ui| {
            egui::Grid::new("cost_grid")
                .num_columns(7)
                .spacing([8.0, 4.0])
                .striped(true)
                .show(ui, |ui| {
                    macro_rules! hdr {
                        ($t:expr) => {
                            ui.label(
                                RichText::new($t)
                                    .monospace()
                                    .size(10.0)
                                    .color(theme::dim_gray()),
                            );
                        };
                    }
                    hdr!("region");
                    hdr!("addr");
                    hdr!("compute");
                    hdr!("I/O wait");
                    hdr!("%C");
                    hdr!("%I");
                    hdr!("total");
                    ui.end_row();

                    for row in &rep.rows {
                        let tot = row.cycles_compute + row.cycles_io_wait;
                        let off = disabled.contains(&row.name);
                        let cell = |text: String,
                                    col: Color32,
                                    ui: &mut Ui|
                         -> egui::Response {
                            ui.label(RichText::new(text).monospace().size(10.5).color(col))
                        };

                        let base = if off {
                            theme::dim_gray()
                        } else {
                            theme::start_green()
                        };
                        let ncol = if off {
                            theme::dim_gray()
                        } else {
                            theme::focus()
                        };
                        let iocol = if off {
                            theme::dim_gray()
                        } else {
                            theme::start_green_dim()
                        };

                        let ntxt = RichText::new(&row.name)
                            .monospace()
                            .size(10.5)
                            .color(base);
                        let hover = format!(
                            "{}\n\nClick to toggle inclusion in %C / %I totals.",
                            row.name
                        );
                        let lr = ui
                            .horizontal(|ui| {
                                ui.set_max_width(140.0);
                                ui.add(Label::new(ntxt).truncate().sense(Sense::click()))
                            })
                            .inner
                            .on_hover_text(hover)
                            .on_hover_cursor(CursorIcon::PointingHand);
                        if lr.clicked() {
                            if off {
                                disabled.remove(&row.name);
                            } else {
                                disabled.insert(row.name.clone());
                            }
                        }

                        cell(format!("0x{:04X}", row.addr), ncol, ui);
                        cell(format!("{}", row.cycles_compute), ncol, ui);
                        cell(format!("{}", row.cycles_io_wait), iocol, ui);

                        let (pct_cs, pct_is) = if off {
                            ("—".to_string(), "—".to_string())
                        } else {
                            (
                                format!(
                                    "{:.1}",
                                    (row.cycles_compute as f32 / denom_inc) * 100.0
                                ),
                                format!(
                                    "{:.1}",
                                    (row.cycles_io_wait as f32 / denom_inc) * 100.0
                                ),
                            )
                        };
                        cell(pct_cs, ncol, ui);
                        cell(pct_is, iocol, ui);
                        cell(format!("{tot}"), ncol, ui);
                        ui.end_row();
                    }
                });
                });
        });

    ui.add_space(8.0);
    ui.add(
        Label::new(
            RichText::new("Static span hints (flash between this label and the next)")
                .monospace()
                .size(11.0)
                .color(theme::focus()),
        )
        .wrap(),
    );
    ui.add_space(4.0);

    let static_w = ui.available_width();
    ScrollArea::vertical()
        .id_salt("cost_static")
        .max_height(140.0)
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.set_max_width(static_w);
            for row in &rep.rows {
                let off = disabled.contains(&row.name);
                let line = if row.static_m == row.static_x {
                    format!(
                        "`{}`  min=max={} cyc  ·  NOPs {}  ·  PUSH {} / POP {}",
                        row.name, row.static_m, row.nop_cnt, row.push_cnt, row.pop_cnt
                    )
                } else {
                    format!(
                        "`{}`  static {}…{} cyc  ·  NOPs {}  ·  PUSH {} / POP {}",
                        row.name, row.static_m, row.static_x, row.nop_cnt, row.push_cnt, row.pop_cnt
                    )
                };
                let line_col = if off {
                    Color32::from_gray(55)
                } else {
                    theme::dim_gray()
                };
                let rt = RichText::new(line).monospace().size(10.0).color(line_col);
                ui.add(Label::new(rt).wrap());
            }
        });

    if !rep.hints.is_empty() {
        ui.add_space(8.0);
        ui.label(
            RichText::new("Suggestions")
                .monospace()
                .size(11.0)
                .color(theme::err_red()),
        );
        ui.add_space(4.0);
        for h in &rep.hints {
            ui.add(
                Label::new(
                    RichText::new(h)
                        .monospace()
                        .size(10.0)
                        .color(theme::err_red()),
                )
                .wrap(),
            );
        }
    }
}

pub fn show_cost_helper(ui: &mut Ui, state: &mut CostHelperState, files: &[(String, String)]) {
    Frame::NONE
        .fill(theme::panel_over_wallpaper(ui.ctx(), theme::panel_deep()))
        .stroke(Stroke::new(0.75, theme::sim_border()))
        .inner_margin(Margin::same(10))
        .show(ui, |ui| {
            let w = ui.available_width();
            ui.set_min_width(w);
            ui.set_max_width(w);

            ui.label(
                RichText::new("[ COST ANALYSIS ]")
                    .monospace()
                    .size(13.0)
                    .color(theme::start_green()),
            );
            ui.add_space(6.0);
            ui.separator();
            ui.add_space(6.0);

            if files.is_empty() {
                ui.add(
                    Label::new(
                        RichText::new("No valid files found in workspace.")
                            .monospace()
                            .size(11.5)
                            .color(theme::dim_gray()),
                    )
                    .wrap(),
                );
                return;
            }

            if state.file_idx != state.prev_file_idx {
                state.disabled_labels.clear();
                state.prev_file_idx = state.file_idx;
            }

            ui.label(
                RichText::new("FILE")
                    .monospace()
                    .size(11.5)
                    .color(theme::start_green_dim()),
            );
            ui.add_space(2.0);
            let sel_name = files
                .get(state.file_idx)
                .map(|(n, _)| n.as_str())
                .unwrap_or("—");
            theme::combo_box("cost_file")
                .width(220.0)
                .selected_text(
                    RichText::new(sel_name)
                        .monospace()
                        .size(11.0)
                        .color(theme::start_green()),
                )
                .show_ui(ui, |ui| {
                    ui.style_mut().visuals.override_text_color = Some(theme::start_green());
                    for (i, (name, _)) in files.iter().enumerate() {
                        ui.selectable_value(
                            &mut state.file_idx,
                            i,
                            RichText::new(name).monospace().size(11.0),
                        );
                    }
                });

            ui.add_space(8.0);

            let do_run = ui
                .add(
                    Button::new(
                        RichText::new("Run analysis")
                            .monospace()
                            .size(12.0)
                            .color(Color32::BLACK),
                    )
                    .fill(theme::start_green())
                    .stroke(Stroke::new(1.0, theme::start_green())),
                )
                .clicked();

            ui.add_space(8.0);
            ui.separator();
            ui.add_space(6.0);

            let source = files
                .get(state.file_idx)
                .map(|(_, c)| c.as_str())
                .unwrap_or("");

            if do_run {
                state.disabled_labels.clear();
                state.cache = Some((state.file_idx, run_analysis(source)));
            }

            if let Some((idx, res)) = &state.cache {
                if *idx != state.file_idx {
                    ui.add(
                        Label::new(
                            RichText::new("File changed since last run — click Run analysis to refresh.")
                                .monospace()
                                .size(10.5)
                                .color(theme::dim_gray()),
                        )
                        .wrap(),
                    );
                    ui.add_space(6.0);
                } else {
                    match res {
                        Ok(rep) => paint_report(ui, rep, &mut state.disabled_labels),
                        Err(e) => {
                            ui.add(
                                Label::new(
                                    RichText::new(e)
                                        .monospace()
                                        .size(11.0)
                                        .color(theme::err_red()),
                                )
                                .wrap(),
                            );
                        }
                    }
                }
            } else {
                ui.add(
                    Label::new(
                        RichText::new("Assembles the file, simulates from reset, attributes cycles to label regions (by PC), and scans each region for NOPs / PUSH–POP balance.")
                            .monospace()
                            .size(10.5)
                            .color(theme::dim_gray()),
                    )
                    .wrap(),
                );
            }
        });
}
