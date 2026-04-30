//! CPU wallpaper effects

use image::{imageops, RgbaImage};

const MAX_SOURCE_EDGE: u32 = 1920;
const MAX_CORNER_RAMP: f32 = 0.5;

fn premultiply_rgba(pixels: &mut [u8]) {
    for p in pixels.chunks_exact_mut(4) {
        let a = f32::from(p[3]) * (1.0 / 255.0);
        p[0] = (f32::from(p[0]) * a).round() as u8;
        p[1] = (f32::from(p[1]) * a).round() as u8;
        p[2] = (f32::from(p[2]) * a).round() as u8;
    }
}

fn unpremultiply_rgba(pixels: &mut [u8]) {
    for p in pixels.chunks_exact_mut(4) {
        let a = f32::from(p[3]);
        if a < 0.5 {
            p[0] = 0;
            p[1] = 0;
            p[2] = 0;
            continue;
        }
        let inv = 255.0 / a;
        p[0] = (f32::from(p[0]) * inv).round().clamp(0.0, 255.0) as u8;
        p[1] = (f32::from(p[1]) * inv).round().clamp(0.0, 255.0) as u8;
        p[2] = (f32::from(p[2]) * inv).round().clamp(0.0, 255.0) as u8;
    }
}

fn limit_max_edge(img: RgbaImage, max_edge: u32) -> RgbaImage {
    let (w, h) = (img.width(), img.height());
    if w == 0 || h == 0 {
        return img;
    }
    let m = w.max(h);
    if m <= max_edge {
        return img;
    }
    let scale    = max_edge as f32 / m as f32;
    let nw = ((w as f32 * scale).round() as u32).max(1);
    let nh = ((h as f32 * scale).round() as u32).max(1);
    imageops::resize(&img, nw, nh, imageops::FilterType::Lanczos3)
}

fn box_blur_separable(
    data:   &mut [u8],
    w:      usize,
    h:      usize,
    r:      usize,
    tmp:    &mut [u8],
    scratch: &mut [i32],
) {
    if r == 0 || w == 0 || h == 0 {
        return;
    }
    let r = r.min(w.max(h));
    for _ in 0..3 {
        blur_horizontal(data, tmp, w, h, r, scratch);
        blur_vertical(tmp, data, w, h, r, scratch);
    }
}

fn blur_horizontal(
    src:     &[u8],
    dst:     &mut [u8],
    w:       usize,
    h:       usize,
    r:       usize,
    scratch: &mut [i32],
) {
    let cap = w + 1;
    debug_assert!(scratch.len() >= cap);
    for y in 0..h {
        let row = y * w * 4;
        for c in 0..4 {
            scratch[0] = 0;
            for x in 0..w {
                let v         = src[row + x * 4 + c] as i32;
                scratch[x + 1] = scratch[x] + v;
            }
            for x in 0..w {
                let x0  = x.saturating_sub(r);
                let x1  = (x + r + 1).min(w);
                let sum = scratch[x1] - scratch[x0];
                let n   = (x1 - x0) as i32;
                let out = (sum / n.max(1)).clamp(0, 255) as u8;
                dst[row + x * 4 + c] = out;
            }
        }
    }
}

fn blur_vertical(
    src:     &[u8],
    dst:     &mut [u8],
    w:       usize,
    h:       usize,
    r:       usize,
    scratch: &mut [i32],
) {
    let cap = h + 1;
    debug_assert!(scratch.len() >= cap);
    for x in 0..w {
        for c in 0..4 {
            scratch[0] = 0;
            for y in 0..h {
                let v         = src[(y * w + x) * 4 + c] as i32;
                scratch[y + 1] = scratch[y] + v;
            }
            for y in 0..h {
                let y0  = y.saturating_sub(r);
                let y1  = (y + r + 1).min(h);
                let sum = scratch[y1] - scratch[y0];
                let n   = (y1 - y0) as i32;
                let out = (sum / n.max(1)).clamp(0, 255) as u8;
                dst[(y * w + x) * 4 + c] = out;
            }
        }
    }
}

fn corner_frost_weight(x: f32, y: f32, w: f32, h: f32, t: f32) -> f32 {
    if t < 0.0001 || w < 1.0 || h < 1.0 {
        return 0.0;
    }
    let m    = w.min(h);
    let d_tl = (x * x + y * y).sqrt();
    let d_tr = ((w - 1.0 - x) * (w - 1.0 - x) + y * y).sqrt();
    let d_bl = (x * x + (h - 1.0 - y) * (h - 1.0 - y)).sqrt();
    let d_br = ((w - 1.0 - x) * (w - 1.0 - x) + (h - 1.0 - y) * (h - 1.0 - y)).sqrt();
    let invr = 1.0 / (t * m * (MAX_CORNER_RAMP) + 1.0);
    let a    = (1.0 - d_tl * invr).max(0.0);
    let b    = (1.0 - d_tr * invr).max(0.0);
    let c0   = (1.0 - d_bl * invr).max(0.0);
    let d0   = (1.0 - d_br * invr).max(0.0);
    t * a.max(b).max(c0).max(d0)
}

fn blur_radius_px(w: u32, h: u32, blur: f32) -> usize {
    if blur < 0.0001 {
        return 0;
    }
    let m = w.min(h) as f32;
    (blur * m * 0.12)
        .round()
        .max(1.0)
        .min(72.0) as usize
}

fn heavy_extra_radius(t: f32) -> usize {
    if t < 0.0001 {
        return 0;
    }
    (4.0 + t * 20.0).round() as usize
}

pub fn apply_wallpaper_effects(img: &mut RgbaImage, blur: f32, corner_smooth: f32) {
    let wu = img.width() as usize;
    let h_u = img.height() as usize;
    if wu == 0 || h_u == 0 {
        return;
    }
    let blur   = blur.clamp(0.0, 1.0);
    let corner = corner_smooth.clamp(0.0, 1.0);
    if blur < 0.0001 && corner < 0.0001 {
        return;
    }
    let w0 = img.width();
    let h0 = img.height();
    let len  = wu * h_u * 4;
    let mut data = std::mem::replace(img, RgbaImage::new(0, 0)).into_raw();
    debug_assert_eq!(data.len(), len);
    premultiply_rgba(&mut data);
    let r0 = blur_radius_px(w0, h0, blur);
    let mut tmp  = vec![0u8; len];
    let mut scratch = vec![0i32; wu.max(h_u) + 1];
    if r0 > 0 {
        box_blur_separable(&mut data, wu, h_u, r0, &mut tmp, &mut scratch);
    }
    if corner > 0.0001 {
        let r_ex   = heavy_extra_radius(corner);
        let r_heavy = (r0 + r_ex).max(1);
        let mut heavy = data.clone();
        box_blur_separable(&mut heavy, wu, h_u, r_heavy, &mut tmp, &mut scratch);
        for y in 0..h_u {
            for x in 0..wu {
                let idx = (y * wu + x) * 4;
                let wgt = corner_frost_weight(
                    x as f32 + 0.5,
                    y as f32 + 0.5,
                    wu as f32,
                    h_u as f32,
                    corner,
                );
                for c in 0..4 {
                    let v  = f32::from(data[idx + c]) * (1.0 - wgt) + f32::from(heavy[idx + c]) * wgt;
                    data[idx + c] = v.round() as u8;
                }
            }
        }
    }
    unpremultiply_rgba(&mut data);
    *img = RgbaImage::from_raw(w0, h0, data).expect("raw buffer");
}

pub fn process_wallpaper_rgba(mut rgba: RgbaImage, blur: f32, corner: f32) -> RgbaImage {
    if rgba.width() == 0 || rgba.height() == 0 {
        return rgba;
    }
    let blur   = blur.clamp(0.0, 1.0);
    let corner = corner.clamp(0.0, 1.0);
    rgba       = limit_max_edge(rgba, MAX_SOURCE_EDGE);
    if blur < 0.0001 && corner < 0.0001 {
        return rgba;
    }
    apply_wallpaper_effects(&mut rgba, blur, corner);
    rgba
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgba;

    #[test]
    fn blur_does_not_panic() {
        let mut i = RgbaImage::from_pixel(8, 8, Rgba([100, 50, 200, 200]));
        apply_wallpaper_effects(&mut i, 0.3, 0.2);
    }
}
