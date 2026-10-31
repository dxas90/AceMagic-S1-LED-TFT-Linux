use super::Widget;
use crate::config::{Color, WidgetConfig};
use crate::render::canvas::Canvas;
use std::collections::HashMap;
use std::path::PathBuf;

/// Renders an SVG icon from the Iconify CDN.
/// `value` is the icon name (without set prefix), e.g. "cpu".
/// Config:
///   "iconSet" — Iconify set name, e.g. "mdi" (default "mdi")
///   "color"   — fill color, e.g. "#ffffff" (default "#ffffff")
///
/// SVGs are cached to `~/.cache/acemagic-s1/icons/{set}/{name}.svg`.
/// On first render the SVG is fetched synchronously; subsequent renders use disk cache.
pub struct IconifyWidget {
    /// (set, name) -> rendered SVG bytes
    svg_cache: HashMap<(String, String), String>,
}

impl IconifyWidget {
    pub fn new() -> Self {
        Self {
            svg_cache: HashMap::new(),
        }
    }

    fn cache_dir(set: &str) -> PathBuf {
        let base = std::env::var("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("/tmp"));
        base.join(".cache")
            .join("acemagic-s1")
            .join("icons")
            .join(set)
    }

    fn cache_path(set: &str, name: &str) -> PathBuf {
        Self::cache_dir(set).join(format!("{}.svg", name))
    }

    /// Load SVG from disk cache or fetch from Iconify CDN.
    /// Returns the SVG text with `currentColor` replaced by `color_hex`.
    fn load_svg(set: &str, name: &str, color_hex: &str) -> Option<String> {
        let path = Self::cache_path(set, name);

        let raw = if path.exists() {
            std::fs::read_to_string(&path).ok()?
        } else {
            // Fetch from Iconify API
            // NOTE: This blocks the calling thread. The draw loop should call this
            // from spawn_blocking so that the tokio executor is not blocked.
            let url = format!("https://api.iconify.design/{}/{}.svg", set, name);
            tracing::debug!("Fetching icon: {url}");
            // Use ureq-style blocking HTTP or reqwest blocking
            // Since we only have async reqwest in Cargo.toml, we do a manual TCP fetch
            // as a fallback. For production, add `reqwest = { features = ["blocking"] }`.
            match fetch_svg_blocking(&url) {
                Some(s) => {
                    if let Some(parent) = path.parent() {
                        let _ = std::fs::create_dir_all(parent);
                    }
                    let _ = std::fs::write(&path, &s);
                    s
                }
                None => {
                    tracing::warn!("Failed to fetch icon {set}/{name} from Iconify");
                    return None;
                }
            }
        };

        // Apply color substitution: replace currentColor with the configured color
        let colored = raw.replace("currentColor", color_hex);
        Some(colored)
    }
}

impl Widget for IconifyWidget {
    fn name(&self) -> &str {
        "iconify"
    }

    fn draw(
        &mut self,
        canvas: &mut Canvas,
        value: &str,
        _min: f64,
        _max: f64,
        config: &WidgetConfig,
    ) -> bool {
        if value.is_empty() {
            return false;
        }

        let icon_set = config.get_str("iconSet", "mdi").to_string();
        let color_str = config.get_str("color", "#ffffff");
        let rect = config.rect;

        let key = (icon_set.clone(), value.to_string());

        if !self.svg_cache.contains_key(&key) {
            match Self::load_svg(&icon_set, value, color_str) {
                Some(svg) => {
                    self.svg_cache.insert(key.clone(), svg);
                }
                None => return false,
            }
        }

        let svg_data = self.svg_cache.get(&key).unwrap();

        let bg = Color::from_hex(config.get_str("background", "#000000")).unwrap_or(Color::BLACK);
        canvas.fill_rect(rect, bg);

        render_svg_into_canvas(svg_data, canvas, rect);

        if config.debug_frame {
            canvas.stroke_rect(rect, Color::new(255, 0, 0), 1.0);
        }
        true
    }
}

/// Render an SVG string into the canvas at the given rect.
pub fn render_svg_into_canvas(svg_data: &str, canvas: &mut Canvas, rect: crate::config::Rect) {
    let opt = usvg::Options::default();
    let Ok(tree) = usvg::Tree::from_str(svg_data, &opt) else {
        tracing::warn!("Failed to parse SVG");
        return;
    };

    let w = rect.width.max(1);
    let h = rect.height.max(1);
    let Some(mut pixmap) = tiny_skia::Pixmap::new(w, h) else {
        return;
    };

    // Scale SVG to fit rect, preserving aspect ratio
    let svg_w = tree.size().width();
    let svg_h = tree.size().height();
    let sx = w as f32 / svg_w;
    let sy = h as f32 / svg_h;
    let scale = sx.min(sy);
    let transform = tiny_skia::Transform::from_scale(scale, scale);

    resvg::render(&tree, transform, &mut pixmap.as_mut());

    canvas.blit_pixmap(&pixmap, rect.x, rect.y);
}

/// Minimal blocking HTTP GET using std::net::TcpStream + rustls or plain HTTP.
/// This is a best-effort implementation for icon fetching.
/// For production reliability, enable the `blocking` feature of reqwest.
fn fetch_svg_blocking(url: &str) -> Option<String> {
    // Use std::process::Command to delegate to curl if available
    let output = std::process::Command::new("curl")
        .args(["-s", "--max-time", "10", url])
        .output()
        .ok()?;
    if output.status.success() {
        let s = String::from_utf8(output.stdout).ok()?;
        if s.trim_start().starts_with('<') {
            Some(s)
        } else {
            None
        }
    } else {
        None
    }
}
