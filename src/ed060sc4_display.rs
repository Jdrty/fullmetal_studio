//! the panel

use eframe::egui::{
    self, Align, Align2, Button, Color32, ColorImage, CornerRadius, Frame, Image, Layout, Margin,
    RichText, Stroke, TextureHandle, TextureOptions, Vec2, Window,
};

use crate::avr::cpu::Cpu;
use crate::ed060sc4_sim::{clear_framebuffer, paper_color, Ed060sc4PortSim};
use crate::welcome::START_GREEN;

pub const WIDTH: usize = 800;
pub const HEIGHT: usize = 600;

/// flat-panel preview: full resolution in memory, scaled on screen (LINEAR) like a physical display
pub struct Ed060sc4Display {
    pub open: bool,
    texture: Option<TextureHandle>,
    framebuffer: ColorImage,
    port_sim: Ed060sc4PortSim,
    fb_dirty: bool,
}

impl Default for Ed060sc4Display {
    fn default() -> Self {
        Self {
            open: false,
            texture: None,
            framebuffer: ColorImage::new([WIDTH, HEIGHT], paper_color()),
            port_sim: Ed060sc4PortSim::default(),
            fb_dirty: true,
        }
    }
}

impl Ed060sc4Display {
    /// call when ED060 reservation is off or MCU is not ATmega128A
    pub fn close_if_unavailable(&mut self, available: bool) {
        if !available {
            self.open = false;
            self.texture = None;
        }
    }

    /// after CPU instructions: updates the framebuffer from GPIO when `enabled` (128A + ED060 toolbar on).
    pub fn drive_from_cpu(&mut self, cpu: &Cpu, enabled: bool) {
        if enabled {
            if self.port_sim.tick(cpu, &mut self.framebuffer) {
                self.fb_dirty = true;
            }
        } else {
            self.port_sim.sync_idle(cpu);
        }
    }

    pub fn on_cpu_reset(&mut self, cpu: &Cpu) {
        self.port_sim.reset();
        self.port_sim.sync_idle(cpu);
        clear_framebuffer(&mut self.framebuffer);
        self.fb_dirty = true;
    }

    pub fn show_window(&mut self, ctx: &egui::Context) {
        if !self.open {
            self.texture = None;
            return;
        }

        if self.texture.is_none() {
            self.texture = Some(ctx.load_texture(
                "ed060sc4_fb",
                self.framebuffer.clone(),
                TextureOptions::LINEAR,
            ));
            self.fb_dirty = false;
        } else if self.fb_dirty {
            if let Some(t) = &mut self.texture {
                t.set(self.framebuffer.clone(), TextureOptions::LINEAR);
            }
            self.fb_dirty = false;
        }

        let tex = self.texture.as_ref().expect("texture just created");

        let screen = ctx.screen_rect();
        let pad = 28.0;
        let max_win = Vec2::new((screen.width() - pad).max(200.0), (screen.height() - pad).max(160.0));
        let body_w = (screen.width() * 0.34).clamp(280.0, 460.0);
        let img_h = body_w * HEIGHT as f32 / WIDTH as f32;
        let chrome = 76.0;
        let default_h = (img_h + chrome).min(max_win.y).max(180.0);
        let default_w = body_w.min(max_win.x);

        let mut close_clicked = false;
        let _ = Window::new("ed060sc4_win")
            .open(&mut self.open)
            .title_bar(false)
            .collapsible(false)
            .resizable(true)
            .default_size(Vec2::new(default_w, default_h))
            .max_size(max_win)
            .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
            .order(egui::Order::Foreground)
            .frame(
                Frame::NONE
                    .fill(Color32::from_rgb(3, 7, 3))
                    .stroke(Stroke::new(2.0, START_GREEN))
                    .inner_margin(Margin::same(8))
                    .corner_radius(CornerRadius::ZERO),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("[ ED060SC4  ·  800×600 ]")
                            .strong()
                            .monospace()
                            .size(14.0)
                            .color(START_GREEN),
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui
                            .add(
                                Button::new(
                                    RichText::new("  X  ")
                                        .monospace()
                                        .size(13.0)
                                        .color(START_GREEN),
                                )
                                .stroke(Stroke::new(1.0, START_GREEN)),
                            )
                            .clicked()
                        {
                            close_clicked = true;
                        }
                    });
                });
                ui.add_space(6.0);
                Frame::NONE
                    .fill(Color32::from_rgb(8, 14, 8))
                    .inner_margin(Margin::same(6))
                    .stroke(Stroke::new(2.0, START_GREEN))
                    .corner_radius(CornerRadius::ZERO)
                    .show(ui, |ui| {
                        let avail = ui.available_size();
                        let aspect = WIDTH as f32 / HEIGHT as f32;
                        let mut w = avail.x;
                        let mut h = w / aspect;
                        if h > avail.y {
                            h = avail.y;
                            w = h * aspect;
                        }
                        ui.vertical_centered(|ui| {
                            Frame::NONE
                                .stroke(Stroke::new(2.0, START_GREEN))
                                .inner_margin(Margin::same(1))
                                .corner_radius(CornerRadius::ZERO)
                                .show(ui, |ui| {
                                    ui.add(
                                        Image::from_texture(tex).fit_to_exact_size(Vec2::new(w, h)),
                                    );
                                });
                        });
                    });
            });
        if close_clicked {
            self.open = false;
        }

        if !self.open {
            self.texture = None;
        }
    }
}
