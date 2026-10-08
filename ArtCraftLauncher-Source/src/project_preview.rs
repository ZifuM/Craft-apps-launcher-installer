//! Read the saved merged canvas without loading or modifying document layers.
use std::{fs::File, io::{Read, Seek, SeekFrom}, path::Path};
use image::{Rgba, RgbaImage};

fn u16be(r: &mut impl Read) -> Option<u16> {
    let mut b = [0; 2]; r.read_exact(&mut b).ok()?; Some(u16::from_be_bytes(b))
}
fn u32be(r: &mut impl Read) -> Option<u32> {
    let mut b = [0; 4]; r.read_exact(&mut b).ok()?; Some(u32::from_be_bytes(b))
}
fn skip(r: &mut File, size: u64) -> Option<()> {
    let end = r.stream_position().ok()?.checked_add(size)?;
    if end > r.metadata().ok()?.len() { return None; }
    r.seek(SeekFrom::Start(end)).ok()?; Some(())
}

pub fn psd_canvas(path: &Path) -> Option<RgbaImage> {
    let mut file = File::open(path).ok()?;
    let mut signature = [0; 4]; file.read_exact(&mut signature).ok()?;
    if &signature != b"8BPS" { return None; }
    let version = u16be(&mut file)?;
    if !matches!(version, 1 | 2) { return None; }
    skip(&mut file, 6)?;
    let channels = u16be(&mut file)? as usize;
    let height = u32be(&mut file)? as usize;
    let width = u32be(&mut file)? as usize;
    let depth = u16be(&mut file)?;
    let mode = u16be(&mut file)?;
    // RGB and grayscale only: unsupported color spaces fall back to embedded previews.
    let colors = match mode { 1 => 1, 3 => 3, _ => return None };
    if width == 0 || height == 0 || width > 300_000 || height > 300_000
        || channels < colors || channels > 56 || !matches!(depth, 8 | 16) { return None; }
    let bytes_per_sample = (depth / 8) as usize;
    let row_bytes = width.checked_mul(bytes_per_sample)?;
    let plane_bytes = row_bytes.checked_mul(height)?;
    let total = plane_bytes.checked_mul(channels)?;
    // Bound decoding work and allocations for damaged or extremely large documents.
    if total > 256 * 1024 * 1024 { return None; }
    for _ in 0..2 { let len = u32be(&mut file)? as u64; skip(&mut file, len)?; }
    let layer_len = if version == 2 {
        let mut b = [0; 8]; file.read_exact(&mut b).ok()?; u64::from_be_bytes(b)
    } else { u32be(&mut file)? as u64 };
    // A negative layer count means the extra merged channel is transparency.
    let layer_start = file.stream_position().ok()?;
    let mut has_alpha = false;
    let length_bytes = if version == 2 { 8 } else { 4 };
    if layer_len >= length_bytes + 2 {
        let info_len = if version == 2 {
            let mut b = [0; 8]; file.read_exact(&mut b).ok()?; u64::from_be_bytes(b)
        } else { u32be(&mut file)? as u64 };
        if info_len >= 2 { has_alpha = (u16be(&mut file)? as i16) < 0; }
    }
    file.seek(SeekFrom::Start(layer_start)).ok()?; skip(&mut file, layer_len)?;
    let compression = u16be(&mut file)?;
    let mut planes = vec![0u8; total];
    match compression {
        0 => file.read_exact(&mut planes).ok()?,
        1 => {
            let rows = height.checked_mul(channels)?;
            let lengths: Vec<usize> = (0..rows).map(|_| {
                if version == 2 { u32be(&mut file).map(|n| n as usize) }
                else { u16be(&mut file).map(|n| n as usize) }
            }).collect::<Option<_>>()?;
            for (row, len) in planes.chunks_exact_mut(row_bytes).zip(lengths) {
                if len > row_bytes.checked_mul(2)?.checked_add(1024)? { return None; }
                let mut packed = vec![0; len]; file.read_exact(&mut packed).ok()?;
                let (mut src, mut dst) = (0usize, 0usize);
                while src < packed.len() {
                    let code = packed[src] as i8; src += 1;
                    if code >= 0 {
                        let n = code as usize + 1;
                        row.get_mut(dst..dst.checked_add(n)?)?.copy_from_slice(packed.get(src..src.checked_add(n)?)?);
                        src += n; dst += n;
                    } else if code != -128 {
                        let n = (1i16 - code as i16) as usize;
                        row.get_mut(dst..dst.checked_add(n)?)?.fill(*packed.get(src)?);
                        src += 1; dst += n;
                    }
                }
                if dst != row_bytes { return None; }
            }
        }
        2 => {
            let mut decoder = flate2::read::ZlibDecoder::new(file);
            decoder.read_exact(&mut planes).ok()?;
        }
        // Prediction and HDR variants retain the embedded-thumbnail fallback.
        _ => return None,
    }
    let scale = (640.0 / width as f64).min(400.0 / height as f64).min(1.0);
    let out_w = (width as f64 * scale).round().max(1.0) as u32;
    let out_h = (height as f64 * scale).round().max(1.0) as u32;
    let mut result = RgbaImage::new(out_w, out_h);
    for (x, y, pixel) in result.enumerate_pixels_mut() {
        let sx = (x as usize * width / out_w as usize).min(width - 1);
        let sy = (y as usize * height / out_h as usize).min(height - 1);
        let offset = (sy * width + sx) * bytes_per_sample;
        let sample = |channel: usize| planes[channel * plane_bytes + offset];
        let rgb = if colors == 1 { [sample(0); 3] } else { [sample(0), sample(1), sample(2)] };
        *pixel = Rgba([rgb[0], rgb[1], rgb[2], if has_alpha && channels > colors { sample(colors) } else { 255 }]);
    }
    Some(result)
}
