use super::Widget;
use crate::config::{Color, WidgetConfig};
use crate::render::canvas::Canvas;

/// Line chart — comma-separated history values rendered as a connected polyline.
/// Config:
///   "fill"      — line color (default "#00e600")
///   "outline"   — background color (default "#000000")
///   "points"    — number of data points to display (default 60)
///   "area"      — bool, fill area under line (default false)
///   "thickness" — line stroke width (default 1.0)
pub struct LineChartWidget {
    last_value: String,
}

impl LineChartWidget {
    pub fn new() -> Self { Self { last_value: String::new() } }
}

impl Widget for LineChartWidget {
    fn name(&self) -> &str { "line_chart" }

    fn draw(&mut self, canvas: &mut Canvas, value: &str, min: f64, max: f64, config: &WidgetConfig) -> bool {
        let changed = value != self.last_value;
        if !changed && config.refresh == 0 { return false; }
        self.last_value = value.to_string();

        let rect      = config.rect;
        let fill_str  = config.get_str("fill",      "#00e600");
        let bg_str    = config.get_str("outline",   "#000000");
        let points    = config.get_f64("points",    60.0) as usize;
        let area      = config.get_bool("area",     false);
        let thickness = config.get_f64("thickness", 1.0) as f32;

        let line_color = Color::from_hex(fill_str).unwrap_or(Color::new(0, 230, 0));
        let bg_color   = Color::from_hex(bg_str).unwrap_or(Color::BLACK);

        canvas.fill_rect(rect, bg_color);

        let all_values: Vec<f64> = value.split(',')
            .filter_map(|s| s.trim().parse().ok())
            .collect();
        let data: &[f64] = if all_values.len() > points {
            &all_values[all_values.len() - points..]
        } else {
            &all_values
        };

        let n = data.len();
        if n < 2 { return changed; }

        let effective_min = min;
        let effective_max = max.max(effective_min + 1.0);
        let span = (effective_max - effective_min) as f32;
        let rx = rect.x as f32;
        let ry = rect.y as f32;
        let rw = rect.width as f32;
        let rh = rect.height as f32;

        let pts: Vec<(f32, f32)> = data.iter().enumerate().map(|(i, &v)| {
            let px = rx + (i as f32 / (n as f32 - 1.0)) * rw;
            let ratio = ((v as f32 - effective_min as f32) / span).clamp(0.0, 1.0);
            let py = ry + rh - ratio * rh;
            (px, py)
        }).collect();

        if area {
            // Semi-transparent fill color
            let area_color = Color::with_alpha(line_color.r, line_color.g, line_color.b, 80);
            canvas.fill_area_under_polyline(&pts, ry + rh, area_color);
        }

        canvas.stroke_polyline(&pts, line_color, thickness);

        if config.debug_frame {
            canvas.stroke_rect(rect, Color::new(255, 0, 0), 1.0);
        }
        changed
    }
}
