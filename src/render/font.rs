use std::collections::HashMap;
use std::sync::Arc;
use fontdue::{Font, FontSettings};
use fontdue::layout::{Layout, CoordinateSystem, TextStyle};
use tiny_skia::Pixmap;
use crate::config::Color;

pub struct FontPool {
    fonts: HashMap<String, Arc<Font>>,
}

impl FontPool {
    pub fn new() -> Self {
        let mut pool = Self { fonts: HashMap::new() };
        pool.load_builtin();
        pool
    }

    pub fn load_file(&mut self, name: &str, path: &str) {
        if let Ok(bytes) = std::fs::read(path) {
            match Font::from_bytes(bytes, FontSettings::default()) {
                Ok(font) => { self.fonts.insert(name.to_string(), Arc::new(font)); }
                Err(e) => { tracing::warn!("Failed to load font {path}: {e}"); }
            }
        }
    }

    pub fn draw_text(
        &self,
        pixmap: &mut Pixmap,
        text: &str, x: f32, y: f32,
        font_name: &str, size: f32,
        color: Color,
    ) {
        let Some(font) = self.get_font(font_name) else { return };
        let mut layout = Layout::new(CoordinateSystem::PositiveYDown);
        layout.append(&[font.as_ref()], &TextStyle::new(text, size, 0));

        let pw = pixmap.width() as i32;
        let ph = pixmap.height() as i32;
        let stride = pw as usize * 4;
        let data = pixmap.data_mut();

        for glyph in layout.glyphs() {
            let (metrics, bitmap) = font.rasterize_config(glyph.key);
            if bitmap.is_empty() || metrics.width == 0 || metrics.height == 0 { continue; }

            let gx = (x + glyph.x) as i32;
            let gy = (y + glyph.y) as i32;

            for row in 0..metrics.height {
                for col in 0..metrics.width {
                    let cx = gx + col as i32;
                    let cy = gy + row as i32;
                    if cx < 0 || cy < 0 || cx >= pw || cy >= ph { continue; }
                    let alpha = bitmap[row * metrics.width + col] as u32;
                    if alpha == 0 { continue; }
                    let off = cy as usize * stride + cx as usize * 4;
                    let inv = 255 - alpha;
                    data[off    ] = ((color.r as u32 * alpha + data[off    ] as u32 * inv) / 255) as u8;
                    data[off + 1] = ((color.g as u32 * alpha + data[off + 1] as u32 * inv) / 255) as u8;
                    data[off + 2] = ((color.b as u32 * alpha + data[off + 2] as u32 * inv) / 255) as u8;
                    data[off + 3] = (alpha + data[off + 3] as u32 * inv / 255).min(255) as u8;
                }
            }
        }
    }

    pub fn measure_width(&self, text: &str, font_name: &str, size: f32) -> f32 {
        let Some(font) = self.get_font(font_name) else { return 0.0 };
        let mut layout = Layout::new(CoordinateSystem::PositiveYDown);
        layout.append(&[font.as_ref()], &TextStyle::new(text, size, 0));
        layout.glyphs().iter().fold(0.0f32, |acc, g| acc.max(g.x + g.width as f32))
    }

    fn get_font(&self, name: &str) -> Option<Arc<Font>> {
        if let Some(f) = self.fonts.get(name) { return Some(f.clone()); }
        // Try stripping CSS size prefix: "18px Arial" -> "Arial"
        if let Some(family) = name.splitn(2, ' ').nth(1) {
            if let Some(f) = self.fonts.get(family.trim()) { return Some(f.clone()); }
        }
        self.fonts.get("default").cloned()
            .or_else(|| self.fonts.values().next().cloned())
    }

    fn load_builtin(&mut self) {
        // Pairs: (alias name, candidate paths)
        let candidates: &[(&str, &[&str])] = &[
            ("default", &[
                "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
                "/usr/share/fonts/dejavu/DejaVuSans.ttf",
                "/usr/share/fonts/TTF/DejaVuSans.ttf",
                "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
                "/usr/share/fonts/liberation/LiberationSans-Regular.ttf",
            ]),
        ];
        for &(name, paths) in candidates {
            for &path in paths {
                if std::path::Path::new(path).exists() {
                    self.load_file(name, path);
                    break;
                }
            }
        }
        // Register "default" font under common CSS family aliases
        if let Some(arc) = self.fonts.get("default").cloned() {
            for alias in &["Arial", "sans-serif", "Helvetica", "Liberation Sans"] {
                self.fonts.entry(alias.to_string()).or_insert_with(|| arc.clone());
            }
        }
    }
}

impl Default for FontPool {
    fn default() -> Self { Self::new() }
}
