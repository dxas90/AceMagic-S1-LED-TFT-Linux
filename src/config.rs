use std::collections::HashMap;
use serde::{Deserialize, Serialize};

// ── Geometry ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl Rect {
    pub fn pixel_count(&self) -> u32 { self.width * self.height }

    /// True if this rect requires splitting for a partial LCD update (max 2048 px).
    pub fn needs_split(&self) -> bool { self.pixel_count() > 2048 }

    /// Clip this rect to a canvas of given dimensions, returning None if fully outside.
    pub fn clip(&self, canvas_w: u32, canvas_h: u32) -> Option<Rect> {
        let x = self.x.max(0) as u32;
        let y = self.y.max(0) as u32;
        let x2 = (self.x + self.width as i32).min(canvas_w as i32).max(0) as u32;
        let y2 = (self.y + self.height as i32).min(canvas_h as i32).max(0) as u32;
        if x2 <= x || y2 <= y { return None; }
        Some(Rect { x: x as i32, y: y as i32, width: x2 - x, height: y2 - y })
    }
}

// ── Color ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const BLACK: Color = Color { r: 0, g: 0, b: 0, a: 255 };
    pub const WHITE: Color = Color { r: 255, g: 255, b: 255, a: 255 };
    pub const TRANSPARENT: Color = Color { r: 0, g: 0, b: 0, a: 0 };

    pub fn new(r: u8, g: u8, b: u8) -> Self { Self { r, g, b, a: 255 } }
    pub fn with_alpha(r: u8, g: u8, b: u8, a: u8) -> Self { Self { r, g, b, a } }

    /// Parse a CSS hex color string (#rgb, #rrggbb, #rrggbbaa).
    pub fn from_hex(s: &str) -> Option<Self> {
        let s = s.trim_start_matches('#');
        match s.len() {
            3 => {
                let r = u8::from_str_radix(&s[0..1], 16).ok()?;
                let g = u8::from_str_radix(&s[1..2], 16).ok()?;
                let b = u8::from_str_radix(&s[2..3], 16).ok()?;
                Some(Self::new(r * 17, g * 17, b * 17))
            }
            6 => {
                let r = u8::from_str_radix(&s[0..2], 16).ok()?;
                let g = u8::from_str_radix(&s[2..4], 16).ok()?;
                let b = u8::from_str_radix(&s[4..6], 16).ok()?;
                Some(Self::new(r, g, b))
            }
            8 => {
                let r = u8::from_str_radix(&s[0..2], 16).ok()?;
                let g = u8::from_str_radix(&s[2..4], 16).ok()?;
                let b = u8::from_str_radix(&s[4..6], 16).ok()?;
                let a = u8::from_str_radix(&s[6..8], 16).ok()?;
                Some(Self { r, g, b, a })
            }
            _ => None,
        }
    }

    /// Encode as RGB565 big-endian (the format the LCD uses on the wire).
    pub fn to_rgb565_be(self) -> u16 {
        ((self.r as u16 & 0xF8) << 8)
            | ((self.g as u16 & 0xFC) << 3)
            | (self.b as u16 >> 3)
    }

    /// Convert to tiny-skia's Color type.
    pub fn to_skia(self) -> tiny_skia::Color {
        tiny_skia::Color::from_rgba8(self.r, self.g, self.b, self.a)
    }
}

// ── TOML AppConfig ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AppConfig {
    #[serde(default)]
    pub device: DeviceConfig,
    #[serde(default)]
    pub display: DisplayConfig,
    #[serde(default)]
    pub web: WebConfig,
    #[serde(default)]
    pub theme: ThemeConfig,
    #[serde(default)]
    pub led: LedConfig,
    #[serde(default)]
    pub sensors: Vec<SensorInstanceConfig>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            device: DeviceConfig::default(),
            display: DisplayConfig::default(),
            web: WebConfig::default(),
            theme: ThemeConfig::default(),
            led: LedConfig::default(),
            sensors: vec![],
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DeviceConfig {
    /// USB HID path string like "1-8:1.1", or "auto" to scan by VID/PID.
    pub lcd: String,
    /// Serial port for the LED strip, e.g. "/dev/ttyUSB0".
    pub led: String,
}

impl Default for DeviceConfig {
    fn default() -> Self {
        Self { lcd: "auto".into(), led: "/dev/ttyUSB0".into() }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DisplayConfig {
    pub poll_ms: u64,
    pub refresh_ms: u64,
    pub heartbeat_ms: u64,
    pub width: u32,
    pub height: u32,
}

impl Default for DisplayConfig {
    fn default() -> Self {
        Self {
            poll_ms: 500,
            refresh_ms: 1600,
            heartbeat_ms: 60_000,
            width: 320,
            height: 170,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WebConfig {
    pub listen: String,
}

impl Default for WebConfig {
    fn default() -> Self { Self { listen: "0.0.0.0:8686".into() } }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ThemeConfig {
    pub file: String,
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self { file: "themes/simple_demo/portrait_simple.json".into() }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LedConfig {
    /// 1=rainbow 2=breathing 3=colors 4=off 5=auto
    pub theme: u8,
    pub speed: u8,
    pub intensity: u8,
}

impl Default for LedConfig {
    fn default() -> Self { Self { theme: 2, speed: 3, intensity: 3 } }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SensorInstanceConfig {
    #[serde(rename = "type")]
    pub sensor_type: String,
    #[serde(flatten)]
    pub params: HashMap<String, toml::Value>,
}

// ── JSON Theme / Screen / Widget ─────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Theme {
    /// "portrait" | "landscape"
    pub orientation: String,
    /// "redraw" | "update"
    pub refresh: String,
    pub screens: Vec<Screen>,
}

impl Theme {
    pub fn is_portrait(&self) -> bool { self.orientation == "portrait" }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Screen {
    pub id: u32,
    pub name: String,
    #[serde(default)]
    pub background: Option<String>,
    #[serde(default)]
    pub wallpaper: Option<String>,
    /// Duration in ms before rotating to next screen. 0 = stay forever.
    #[serde(default)]
    pub duration: u64,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub led_config: Option<ScreenLedConfig>,
    #[serde(default)]
    pub widgets: Vec<WidgetConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ScreenLedConfig {
    pub theme: u8,
    pub intensity: u8,
    pub speed: u8,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WidgetConfig {
    pub id: u32,
    pub group: u32,
    pub name: String,
    pub rect: Rect,
    /// If true, `value` is a sensor name; otherwise a literal string.
    #[serde(default)]
    pub sensor: bool,
    pub value: String,
    #[serde(default)]
    pub format: String,
    /// Minimum re-render interval in ms; 0 = only once.
    #[serde(default)]
    pub refresh: u64,
    #[serde(default)]
    pub debug_frame: bool,
    /// All widget-type-specific fields preserved as raw JSON for lossless round-trip.
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

impl WidgetConfig {
    pub fn get_str<'a>(&'a self, key: &str, default: &'a str) -> &'a str {
        self.extra.get(key).and_then(|v| v.as_str()).unwrap_or(default)
    }

    pub fn get_f64(&self, key: &str, default: f64) -> f64 {
        self.extra.get(key).and_then(|v| v.as_f64()).unwrap_or(default)
    }

    pub fn get_bool(&self, key: &str, default: bool) -> bool {
        self.extra.get(key).and_then(|v| v.as_bool()).unwrap_or(default)
    }
}

fn default_true() -> bool { true }

// ── Split large rects into ≤2048-pixel sub-rects ──────────────────────────────

/// Split a rect into sub-rects each fitting within the LCD partial-update limit
/// of 2048 pixels (4096 bytes). Mirrors calc_update_region() from main.js.
pub fn split_update_region(rect: Rect) -> Vec<Rect> {
    const MAX_PIXELS: u32 = 2048;
    if rect.pixel_count() <= MAX_PIXELS {
        return vec![rect];
    }
    let sq = (MAX_PIXELS as f64).sqrt();
    let rows = (rect.height as f64 / sq).ceil() as u32;
    let cols = (rect.width as f64 / sq).ceil() as u32;
    let aw = (rect.width + cols - 1) / cols;
    let ah = (rect.height + rows - 1) / rows;
    let mut result = Vec::new();
    for row in 0..rows {
        for col in 0..cols {
            let x = rect.x + (col * aw) as i32;
            let y = rect.y + (row * ah) as i32;
            // Clamp width/height to the actual remaining bounds
            let x2 = (rect.x + rect.width as i32).min(x + aw as i32);
            let y2 = (rect.y + rect.height as i32).min(y + ah as i32);
            let w = (x2 - x).max(0) as u32;
            let h = (y2 - y).max(0) as u32;
            if w > 0 && h > 0 {
                result.push(Rect { x, y, width: w, height: h });
            }
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_hex_colors() {
        let c = Color::from_hex("#ff8800").unwrap();
        assert_eq!(c.r, 0xff);
        assert_eq!(c.g, 0x88);
        assert_eq!(c.b, 0x00);
    }

    #[test]
    fn parse_hex_short() {
        let c = Color::from_hex("#f80").unwrap();
        assert_eq!(c.r, 0xff);
        assert_eq!(c.g, 0x88);
        assert_eq!(c.b, 0x00);
    }

    #[test]
    fn split_region_small() {
        let r = Rect { x: 0, y: 0, width: 40, height: 40 };
        let rects = split_update_region(r);
        assert_eq!(rects.len(), 1);
    }

    #[test]
    fn split_region_large() {
        let r = Rect { x: 0, y: 0, width: 320, height: 170 };
        let rects = split_update_region(r);
        for sr in &rects {
            assert!(sr.pixel_count() <= 2048, "sub-rect too large: {sr:?}");
        }
        // Total pixels should equal original
        let total: u32 = rects.iter().map(|r| r.pixel_count()).sum();
        assert_eq!(total, 320 * 170);
    }

    #[test]
    fn rect_clip() {
        let r = Rect { x: -10, y: -10, width: 50, height: 50 };
        let clipped = r.clip(320, 170).unwrap();
        assert_eq!(clipped.x, 0);
        assert_eq!(clipped.y, 0);
        assert_eq!(clipped.width, 40);
        assert_eq!(clipped.height, 40);
    }
}
