//! LCD display driver for the AceMagic S1 (Holtek HT32 via USB HID).
//!
//! Protocol reference from s1panel/lcd_device.js and the project README:
//! - VID: 0x04D9, PID: 0xFD01
//! - Every write = 4105 bytes: [0x00 report_id][8-byte header][4096-byte data]
//! - Pixels: RGB565 big-endian, 320×170 = 108800 bytes total
//! - Full redraw = 27 sequential chunks (26 × 4096 bytes + 1 × 2304 bytes)
//! - Partial update = dirty-rect command, max 2048 pixels per call

use hidapi::{HidApi, HidDevice};
use crate::config::Rect;

pub const LCD_VID: u16 = 0x04D9;
pub const LCD_PID: u16 = 0xFD01;

const REPORT_SIZE: usize = 1;     // HID report ID byte (always 0x00)
const HEADER_SIZE: usize = 8;
const DATA_SIZE:   usize = 4096;
const BUFFER_SIZE: usize = REPORT_SIZE + HEADER_SIZE + DATA_SIZE; // 4105

const SIGNATURE:   u8 = 0x55;
const CMD_CONFIG:  u8 = 0xA1;  // orientation + heartbeat
const CMD_REFRESH: u8 = 0xA2;  // partial dirty-rect update
const CMD_REDRAW:  u8 = 0xA3;  // full-screen redraw

const CHUNK_COUNT:      usize = 27;
const FINAL_CHUNK_IDX:  usize = CHUNK_COUNT - 1;
const FINAL_CHUNK_SIZE: usize = 2304; // bytes in last chunk (1152 pixels × 2)

pub struct LcdDevice {
    device: HidDevice,
}

impl LcdDevice {
    /// Open the LCD by its HID path string (e.g. "1-8:1.1"),
    /// or scan by VID/PID if path == "auto".
    pub fn open(api: &HidApi, path: &str) -> crate::error::Result<Self> {
        let device = if path == "auto" {
            api.open(LCD_VID, LCD_PID)?
        } else {
            let cpath = std::ffi::CString::new(path)
                .map_err(|_| crate::error::Error::Config("invalid HID path".into()))?;
            api.open_path(cpath.as_c_str())?
        };
        Ok(Self { device })
    }

    /// Set display orientation. Call once on startup before any drawing.
    pub fn set_orientation(&self, portrait: bool) -> crate::error::Result<()> {
        let mut buf = [0u8; BUFFER_SIZE];
        buf[1] = SIGNATURE;
        buf[2] = CMD_CONFIG;
        buf[3] = 0xF1; // LCD_ORIENTATION sub-command
        buf[4] = if portrait { 0x02 } else { 0x01 };
        self.device.write(&buf)?;
        Ok(())
    }

    /// Send heartbeat / set internal clock. Call every heartbeat_ms milliseconds.
    pub fn heartbeat(&self) -> crate::error::Result<()> {
        use chrono::Timelike;
        let now = chrono::Local::now();
        let mut buf = [0u8; BUFFER_SIZE];
        buf[1] = SIGNATURE;
        buf[2] = CMD_CONFIG;
        buf[3] = 0xF2; // LCD_SET_TIME sub-command
        buf[4] = now.hour() as u8;
        buf[5] = now.minute() as u8;
        buf[6] = now.second() as u8;
        self.device.write(&buf)?;
        Ok(())
    }

    /// Partial update: send a dirty rectangle of pixels to the device.
    ///
    /// `pixels` must contain exactly `rect.width * rect.height` RGB565 big-endian values.
    /// Maximum: 2048 pixels (4096 bytes). Call `split_update_region` first for larger rects.
    pub fn partial_update(&self, rect: Rect, pixels: &[u16]) -> crate::error::Result<()> {
        debug_assert_eq!(
            pixels.len(),
            (rect.width * rect.height) as usize,
            "pixel count mismatch: expected {}, got {}",
            rect.width * rect.height,
            pixels.len()
        );
        debug_assert!(pixels.len() <= 2048, "rect too large for partial update");

        let mut buf = [0u8; BUFFER_SIZE];
        buf[1] = SIGNATURE;
        buf[2] = CMD_REFRESH;
        // x and y as little-endian u16; they sit at header offsets 2-3 and 4-5,
        // which map to buf[3..5] and buf[5..7] (offset by 1 for the report ID byte).
        buf[3..5].copy_from_slice(&(rect.x as u16).to_le_bytes());
        buf[5..7].copy_from_slice(&(rect.y as u16).to_le_bytes());
        buf[7] = rect.width as u8;
        buf[8] = rect.height as u8;
        // Pixel data starts right after the 8-byte header (+ 1 report ID byte)
        let data_start = REPORT_SIZE + HEADER_SIZE;
        for (i, &px) in pixels.iter().enumerate() {
            let off = data_start + i * 2;
            buf[off..off + 2].copy_from_slice(&px.to_be_bytes());
        }
        self.device.write(&buf)?;
        Ok(())
    }

    /// Full-screen redraw: sends all 27 chunks covering the complete 320×170 framebuffer.
    ///
    /// `pixels` must be exactly 320 × 170 = 54400 RGB565 big-endian values (108800 bytes total).
    pub fn full_redraw(&self, pixels: &[u16]) -> crate::error::Result<()> {
        debug_assert_eq!(
            pixels.len(),
            320 * 170,
            "full redraw needs exactly {} pixels, got {}",
            320 * 170,
            pixels.len()
        );

        let mut buf = [0u8; BUFFER_SIZE];
        buf[1] = SIGNATURE;
        buf[2] = CMD_REDRAW;

        for idx in 0..CHUNK_COUNT {
            let sub_cmd: u8 = match idx {
                0               => 0xF0, // LCD_REDRAW_START
                FINAL_CHUNK_IDX => 0xF2, // LCD_REDRAW_END
                _               => 0xF1, // LCD_REDRAW_CONTINUE
            };
            let chunk_bytes = if idx < FINAL_CHUNK_IDX { DATA_SIZE } else { FINAL_CHUNK_SIZE };
            let pixel_start = idx * DATA_SIZE / 2;
            let pixel_count = chunk_bytes / 2;

            buf[3] = sub_cmd;
            buf[4] = (idx + 1) as u8; // 1-based sequence number (0x01 .. 0x1B)
            // Byte offset of this chunk in the full framebuffer (big-endian u16).
            // Firmware ignores this and uses the sequence number, but we send it anyway.
            let offset = (idx * DATA_SIZE) as u16;
            buf[6..8].copy_from_slice(&offset.to_be_bytes());

            let data_start = REPORT_SIZE + HEADER_SIZE;
            // Zero leftover bytes from previous chunk
            buf[data_start..].fill(0);
            for (i, &px) in pixels[pixel_start..pixel_start + pixel_count].iter().enumerate() {
                let off = data_start + i * 2;
                buf[off..off + 2].copy_from_slice(&px.to_be_bytes());
            }
            self.device.write(&buf)?;
        }
        Ok(())
    }
}

/// Convert a single RGBA8888 pixel to RGB565 big-endian (the LCD wire format).
/// Alpha is ignored — the background is assumed to be black.
#[inline(always)]
pub fn rgba_to_rgb565_be(r: u8, g: u8, b: u8) -> u16 {
    ((r as u16 & 0xF8) << 8) | ((g as u16 & 0xFC) << 3) | (b as u16 >> 3)
}

/// Convert a 320×170 RGBA8888 row-major pixmap to a flat array of RGB565 big-endian values.
///
/// The canvas is always 320×170 regardless of display orientation; portrait mode is
/// handled by the LCD firmware via `set_orientation()`, so no software rotation is needed.
/// The `portrait` parameter is retained for API compatibility but does not change the output.
pub fn pixmap_to_rgb565(data: &[u8], _portrait: bool) -> Vec<u16> {
    const W: usize = 320;
    const H: usize = 170;
    let stride = W * 4;
    let mut out = Vec::with_capacity(W * H);
    for row in 0..H {
        for col in 0..W {
            let off = row * stride + col * 4;
            out.push(rgba_to_rgb565_be(data[off], data[off + 1], data[off + 2]));
        }
    }
    out
}

/// Translate a dirty-rect from logical canvas coords to physical device coords
/// when the display is in portrait mode.
///
/// Mirrors the JavaScript:
///   `{ x: rect.y, y: (height - (rect.x + rect.width)), width: rect.height, height: rect.width }`
pub fn translate_rect_portrait(rect: Rect, canvas_height: u32) -> Rect {
    Rect {
        x: rect.y,
        y: canvas_height as i32 - (rect.x + rect.width as i32),
        width: rect.height,
        height: rect.width,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rgb565_black() {
        assert_eq!(rgba_to_rgb565_be(0, 0, 0), 0x0000);
    }

    #[test]
    fn rgb565_white() {
        assert_eq!(rgba_to_rgb565_be(255, 255, 255), 0xFFFF);
    }

    #[test]
    fn rgb565_red() {
        // r=0xF8 → r5=0b11111 → top 5 bits of high byte = 0b11111000 = 0xF8xx
        let px = rgba_to_rgb565_be(0xF8, 0, 0);
        assert_eq!(px, 0xF800);
    }

    #[test]
    fn rgb565_blue() {
        // b=0xFF → b5=0b11111 → low 5 bits = 0x001F
        let px = rgba_to_rgb565_be(0, 0, 0xFF);
        assert_eq!(px, 0x001F);
    }

    #[test]
    fn portrait_pixmap_pixel_count() {
        let data = vec![0u8; 320 * 170 * 4];
        let out = pixmap_to_rgb565(&data, true);
        assert_eq!(out.len(), 320 * 170);
    }

    #[test]
    fn landscape_pixmap_pixel_count() {
        let data = vec![0u8; 320 * 170 * 4];
        let out = pixmap_to_rgb565(&data, false);
        assert_eq!(out.len(), 320 * 170);
    }

    #[test]
    fn portrait_rect_translate() {
        // Widget at x=10, y=20, w=30, h=40 on a 170-high canvas
        let r = Rect { x: 10, y: 20, width: 30, height: 40 };
        let t = translate_rect_portrait(r, 170);
        assert_eq!(t.x, 20);
        assert_eq!(t.y, 170 - (10 + 30)); // 130
        assert_eq!(t.width, 40);
        assert_eq!(t.height, 30);
    }
}
