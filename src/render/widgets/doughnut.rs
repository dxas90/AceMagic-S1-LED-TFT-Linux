use super::Widget;
use crate::config::{Color, WidgetConfig};
use crate::render::canvas::Canvas;

/// Doughnut (ring) chart showing a percentage arc.
/// Config:
///   "used"         — filled arc color (default "#00e600")
///   "free"         — remaining arc color (default "#333333")
///   "rotation"     — arc start angle in degrees, 0=right, 90=down (default 225)
///   "cutout"       — inner radius as percent of outer, e.g. "80%" or 80 (default "80%")
///   "circumference"— total arc sweep in degrees (default 270)
/// `value` is a percentage 0–100.
pub struct DoughnutWidget {
    last_value: f64,
}

impl DoughnutWidget {
    pub fn new() -> Self {
        Self { last_value: -1.0 }
    }
}

impl Widget for DoughnutWidget {
    fn name(&self) -> &str {
        "doughnut"
    }

    fn draw(
        &mut self,
        canvas: &mut Canvas,
        value: &str,
        _min: f64,
        _max: f64,
        config: &WidgetConfig,
    ) -> bool {
        let pct: f64 = value.parse::<f64>().unwrap_or(0.0).clamp(0.0, 100.0);
        let changed = (pct - self.last_value).abs() > 0.4;
        if !changed && config.refresh == 0 {
            return false;
        }
        self.last_value = pct;

        let rect = config.rect;
        let used_str = config.get_str("used", "#00e600");
        let free_str = config.get_str("free", "#333333");
        let bg_str = config.get_str("background", "#000000");
        let rot_deg = config.get_f64("rotation", 225.0) as f32;
        let circ_deg = config.get_f64("circumference", 270.0) as f32;
        let cutout_s = config.get_str("cutout", "80%");

        let used_color = Color::from_hex(used_str).unwrap_or(Color::new(0, 230, 0));
        let free_color = Color::from_hex(free_str).unwrap_or(Color::new(51, 51, 51));
        let bg_color = Color::from_hex(bg_str).unwrap_or(Color::BLACK);

        let cutout: f32 = if cutout_s.ends_with('%') {
            cutout_s
                .trim_end_matches('%')
                .parse::<f32>()
                .unwrap_or(80.0)
                / 100.0
        } else {
            cutout_s.parse::<f32>().unwrap_or(0.8)
        };

        canvas.fill_rect(rect, bg_color);

        let cx = rect.x as f32 + rect.width as f32 / 2.0;
        let cy = rect.y as f32 + rect.height as f32 / 2.0;
        let r_outer = (rect.width.min(rect.height) as f32 / 2.0) * 0.95;
        let r_inner = r_outer * cutout.clamp(0.0, 0.95);

        let to_rad = std::f32::consts::PI / 180.0;
        let start_rad = rot_deg * to_rad;
        let total_rad = circ_deg * to_rad;
        let used_rad = total_rad * (pct / 100.0) as f32;
        let free_rad = total_rad - used_rad;

        // Draw free (background) arc first
        if free_rad > 0.001 {
            canvas.fill_arc(
                cx,
                cy,
                r_outer,
                r_inner,
                start_rad + used_rad,
                free_rad,
                free_color,
            );
        }
        // Draw used arc on top
        if used_rad > 0.001 {
            canvas.fill_arc(cx, cy, r_outer, r_inner, start_rad, used_rad, used_color);
        }

        if config.debug_frame {
            canvas.stroke_rect(rect, Color::new(255, 0, 0), 1.0);
        }
        changed
    }
}
