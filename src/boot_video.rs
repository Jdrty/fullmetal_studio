//! Full-window boot sequence: first 114 frames of `assets/videos/fake.mov` via ffmpeg raw RGBA.

use std::io::{BufReader, Read};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use egui::{Color32, ColorImage, CornerRadius, TextureHandle, TextureOptions};

/// H.264 stream size from `ffprobe` (must match ffmpeg output when not scaling).
const WIDTH: usize = 1920;
const HEIGHT: usize = 1080;
const FRAME_COUNT: u32 = 114;
/// 30 fps (integer nanoseconds; ~3.3 ns drift per frame vs exact 1/30 s).
const FRAME_INTERVAL: Duration = Duration::from_nanos(1_000_000_000 / 30);

enum Phase {
    NeedSpawn,
    Decoding {
        #[allow(dead_code)]
        child: Child,
        reader: BufReader<std::process::ChildStdout>,
    },
    Black,
    Done,
}

pub struct BootVideo {
    phase: Phase,
    /// Frames advanced (video pixels or black), 0..=114.
    shown: u32,
    texture: Option<TextureHandle>,
    buf: Vec<u8>,
    /// Earliest wall time we advance to the next decoded frame (30 fps cadence).
    next_frame_deadline: Instant,
}

impl BootVideo {
    pub fn new() -> Self {
        Self {
            phase: Phase::NeedSpawn,
            shown: 0,
            texture: None,
            buf: Vec::new(),
            next_frame_deadline: Instant::now(),
        }
    }

    pub fn done(&self) -> bool {
        matches!(self.phase, Phase::Done)
    }

    fn clip_path() -> PathBuf {
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/videos/fake.mov");
        if manifest.is_file() {
            manifest
        } else {
            PathBuf::from("assets/videos/fake.mov")
        }
    }

    fn try_spawn_decoder() -> Result<(Child, BufReader<std::process::ChildStdout>), String> {
        let path = Self::clip_path();
        if !path.is_file() {
            return Err(format!("boot video missing: {}", path.display()));
        }
        let path_str = path
            .to_str()
            .ok_or_else(|| "boot video path is not valid UTF-8".to_string())?;

        let mut child = Command::new("ffmpeg")
            .arg("-hide_banner")
            .arg("-loglevel")
            .arg("error")
            .arg("-i")
            .arg(path_str)
            .arg("-an")
            .arg("-frames:v")
            .arg(FRAME_COUNT.to_string())
            .arg("-f")
            .arg("rawvideo")
            .arg("-pix_fmt")
            .arg("rgba")
            .arg("-")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| {
                format!(
                    "could not run `ffmpeg` (is it installed and on PATH?): {e}"
                )
            })?;

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| "ffmpeg: no stdout".to_string())?;

        Ok((child, BufReader::new(stdout)))
    }

    fn cleanup_decoder(&mut self) {
        let prev = std::mem::replace(&mut self.phase, Phase::Black);
        match prev {
            Phase::Decoding { mut child, .. } => {
                let _ = child.kill();
                let _ = child.wait();
                self.phase = Phase::Black;
            }
            other => {
                self.phase = other;
            }
        }
    }

    fn draw_layer(&self, ui: &mut egui::Ui, rect: egui::Rect) {
        ui.painter()
            .rect_filled(rect, CornerRadius::ZERO, Color32::BLACK);

        if let Some(tex) = &self.texture {
            let [tw, th] = tex.size();
            let tw = tw as f32;
            let th = th as f32;
            let scale = (rect.width() / tw).min(rect.height() / th);
            let size = egui::vec2(tw * scale, th * scale);
            let origin = rect.center() - size * 0.5;
            let image_rect = egui::Rect::from_min_size(origin, size);
            ui.put(
                image_rect,
                egui::Image::from_texture(tex).fit_to_exact_size(size),
            );
        }
    }

    /// One egui frame of the boot sequence: advance by one logical frame at 30 fps, then repaint until done.
    pub fn show(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        if matches!(self.phase, Phase::Done) {
            return;
        }

        if self.shown >= FRAME_COUNT {
            self.cleanup_decoder();
            self.phase = Phase::Done;
            self.texture = None;
            return;
        }

        let rect = ui.max_rect();
        let now = Instant::now();
        if now < self.next_frame_deadline {
            self.draw_layer(ui, rect);
            ctx.request_repaint_after(self.next_frame_deadline - now);
            return;
        }

        let frame_bytes = WIDTH * HEIGHT * 4;
        self.buf.resize(frame_bytes, 0);

        if matches!(self.phase, Phase::NeedSpawn) {
            match Self::try_spawn_decoder() {
                Ok((child, reader)) => {
                    self.phase = Phase::Decoding { child, reader };
                }
                Err(e) => {
                    eprintln!("lainstudio boot: {e}");
                    self.phase = Phase::Black;
                }
            }
        }

        let mut pixel_ok = false;
        if let Phase::Decoding { reader, .. } = &mut self.phase {
            match reader.read_exact(&mut self.buf) {
                Ok(()) => pixel_ok = true,
                Err(e) => {
                    eprintln!("lainstudio boot: stream ended early ({e})");
                    self.texture = None;
                    self.cleanup_decoder();
                    self.phase = Phase::Black;
                }
            }
        }

        if pixel_ok {
            let image = ColorImage::from_rgba_unmultiplied([WIDTH, HEIGHT], &self.buf);
            match &mut self.texture {
                Some(t) => t.set(image, TextureOptions::LINEAR),
                None => {
                    self.texture = Some(ctx.load_texture(
                        "boot_video_splash",
                        image,
                        TextureOptions::LINEAR,
                    ));
                }
            }
        }

        self.draw_layer(ui, rect);

        self.shown += 1;
        self.next_frame_deadline = now + FRAME_INTERVAL;
        ctx.request_repaint();
    }
}

impl Drop for BootVideo {
    fn drop(&mut self) {
        let prev = std::mem::replace(&mut self.phase, Phase::Black);
        if let Phase::Decoding { mut child, .. } = prev {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}
