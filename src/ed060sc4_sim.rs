//! ED060SC4 GPIO protocol
//  this is kinda pointless and unrelated to avr, but the whole reason
//  for me making this is to simulate the panel, so ill keep it in here anyway.
//  its an easter egg i guess, why wouldnt you want visual output

use eframe::egui::{Color32, ColorImage};

use crate::avr::cpu::Cpu;
use crate::avr::io_map;

const WIDTH: usize = 800;
const HEIGHT: usize = 600;
const ROW_BYTES: usize = WIDTH / 4;

#[inline]
fn pin_level(cpu: &Cpu, ddr: u16, port: u16, pin: u16, bit: u8) -> bool {
    let ddr_v = cpu.read_mem(ddr);
    let port_v = cpu.read_mem(port);
    let pin_v = cpu.read_mem(pin);
    if (ddr_v >> bit) & 1 != 0 {
        (port_v >> bit) & 1 != 0
    } else {
        (pin_v >> bit) & 1 != 0
    }
}

#[derive(Clone, Default)]
struct Levels {
    sph:   bool,
    cl:    bool,
    le:    bool,
    ckv:   bool,
    spv:   bool,
    oe:    bool,
    gmode: bool,
    pwr:   bool,
    pd:    u8,
}

impl Levels {
    fn read(cpu: &Cpu) -> Self {
        Self {
            sph:   pin_level(cpu, io_map::DDRB, io_map::PORTB, io_map::PINB, 0),
            cl:    pin_level(cpu, io_map::DDRB, io_map::PORTB, io_map::PINB, 6),
            le:    pin_level(cpu, io_map::DDRB, io_map::PORTB, io_map::PINB, 5),
            ckv:   pin_level(cpu, io_map::DDRE, io_map::PORTE, io_map::PINE, 4),
            spv:   pin_level(cpu, io_map::DDRE, io_map::PORTE, io_map::PINE, 3),
            oe:    pin_level(cpu, io_map::DDRE, io_map::PORTE, io_map::PINE, 2),
            gmode: pin_level(cpu, io_map::DDRE, io_map::PORTE, io_map::PINE, 5),
            pwr:   pin_level(cpu, io_map::DDRB, io_map::PORTB, io_map::PINB, 7),
            pd:    cpu.read_mem(io_map::PORTD),
        }
    }
}

#[derive(Clone)]
pub struct Ed060sc4PortSim {
    prev: Levels,
    shift_bytes: [u8; ROW_BYTES],
    shift_count: usize,
    latched_row: [u8; ROW_BYTES],
    row: usize,
    /// OE fell and row pixels were applied; next CKV↓ (with SPV high) advances `row`
    pending_row_advance: bool,
}

impl Default for Ed060sc4PortSim {
    fn default() -> Self {
        Self {
            prev: Levels::default(),
            shift_bytes: [0u8; ROW_BYTES],
            shift_count: 0,
            latched_row: [0u8; ROW_BYTES],
            row: 0,
            pending_row_advance: false,
        }
    }
}

impl Ed060sc4PortSim {
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// after each CPU instruction while ED060 mode is active. returns true if framebuffer changed
    pub fn tick(&mut self, cpu: &Cpu, fb: &mut ColorImage) -> bool {
        let p = Levels::read(cpu);
        let prev = &self.prev;

        let sph_low_scan = !p.sph;
        let mut dirty = false;

        // SPH low → new horizontal scan (shift count reset on falling edge of SPH)
        if prev.sph && !p.sph {
            self.shift_count = 0;
        }

        // CL rising while SPH low: sample PD (one byte = 4 pixels @ 2 bpp)
        if sph_low_scan && !prev.cl && p.cl && self.shift_count < ROW_BYTES {
            self.shift_bytes[self.shift_count] = p.pd;
            self.shift_count += 1;
        }

        // LE falling: move shift register into row latch
        if prev.le && !p.le {
            self.latched_row.copy_from_slice(&self.shift_bytes);
        }

        // OE falling: "ink" phase ends; commit latched row to framebuffer at current row
        let drive_ok = p.gmode && p.pwr && p.spv;
        if prev.oe && !p.oe && drive_ok && self.row < HEIGHT {
            dirty |= apply_latched_row(fb, self.row, &self.latched_row);
            self.pending_row_advance = true;
            // next row’s shift can start even if firmware leaves SPH low between lines
            self.shift_count = 0;
        }

        // CKV falling: vertical timing
        if prev.ckv && !p.ckv {
            if !p.spv {
                // vertical init / frame sync while SPV low: arm row 0
                self.row = 0;
                self.pending_row_advance = false;
            } else if drive_ok && self.pending_row_advance {
                self.row = (self.row + 1).min(HEIGHT - 1);
                self.pending_row_advance = false;
            }
        }

        self.prev = p;
        dirty
    }

    /// while ED060 mode is off: track levels so re-enabling does not synthesize edges
    pub fn sync_idle(&mut self, cpu: &Cpu) {
        self.prev = Levels::read(cpu);
    }
}

/// 2 bpp nibble: 00 hold, 01 black, 10 white, 11 reserved (treated as hold).
fn apply_latched_row(fb: &mut ColorImage, row: usize, latched: &[u8; ROW_BYTES]) -> bool {
    let mut changed = false;
    let base = row * WIDTH;
    for (bi, &byte) in latched.iter().enumerate() {
        for q in 0..4 {
            let col = bi * 4 + q;
            if col >= WIDTH {
                break;
            }
            let pair = (byte >> (q * 2)) & 3;
            let idx = base + col;
            let new_px = match pair {
                0 => None,
                1 => Some(Color32::BLACK),
                2 => Some(Color32::WHITE),
                _ => None,
            };
            if let Some(c) = new_px {
                if fb.pixels[idx] != c {
                    fb.pixels[idx] = c;
                    changed = true;
                }
            }
        }
    }
    changed
}

pub fn paper_color() -> Color32 {
    Color32::from_rgb(244, 242, 236)
}

pub fn clear_framebuffer(fb: &mut ColorImage) {
    let c = paper_color();
    for px in &mut fb.pixels {
        *px = c;
    }
}
