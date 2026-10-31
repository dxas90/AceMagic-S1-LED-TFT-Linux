use super::Widget;
use crate::config::{Color, Rect, WidgetConfig};
use crate::render::canvas::Canvas;

/// Horizontal progress bar with dark gradient background and green fill.
/// `value` is a percentage string 0–100.
pub struct CustomBarWidget {
    last_value: f64,
}

impl CustomBarWidget {
    pub fn new() -> Self { Self { last_value: -1.0 } }
}

impl Widget for CustomBarWidget {
    fn name(&self) -> &str { "custom_bar" }

    fn draw(&mut self, canvas: &mut Canvas, value: &str, _min: f64, _max: f64, config: &WidgetConfig) -> bool {
        let pct: f64 = value.parse::<f64>().unwrap_or(0.0).clamp(0.0, 100.0);
        let changed = (pct - self.last_value).abs() > 0.4;
        if !changed && config.refresh == 0 { return false; }
        self.last_value = pct;

        let rect = config.rect;
        let bg_left  = Color::from_hex(config.get_str("background_left",  "#404040")).unwrap_or(Color::new(64, 64, 64));
        let bg_right = Color::from_hex(config.get_str("background_right", "#000000")).unwrap_or(Color::BLACK);
        let fill_left  = Color::from_hex(config.get_str("fill_left",  "#90ee90")).unwrap_or(Color::new(144, 238, 144));
        let fill_right = Color::from_hex(config.get_str("fill_right", "#006400")).unwrap_or(Color::new(0, 100, 0));

        canvas.fill_rect_gradient_h(rect, bg_left, bg_right);

        let fill_w = ((rect.width as f64 * pct / 100.0).round() as u32).min(rect.width);
        if fill_w > 0 {
            canvas.fill_rect_gradient_h(
                Rect { x: rect.x, y: rect.y, width: fill_w, height: rect.height },
                fill_left, fill_right,
            );
        }

        if config.debug_frame {
            canvas.stroke_rect(rect, Color::new(255, 0, 0), 1.0);
        }
        changed
    }
}
