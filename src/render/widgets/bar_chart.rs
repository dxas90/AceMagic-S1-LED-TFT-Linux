use super::Widget;
use crate::config::{Color, Rect, WidgetConfig};
use crate::render::canvas::Canvas;

/// Bar chart — comma-separated values, each rendered as a vertical or horizontal bar.
pub struct BarChartWidget {
    last_value: String,
}

impl BarChartWidget {
    pub fn new() -> Self {
        Self {
            last_value: String::new(),
        }
    }
}

impl Widget for BarChartWidget {
    fn name(&self) -> &str {
        "bar_chart"
    }

    fn draw(
        &mut self,
        canvas: &mut Canvas,
        value: &str,
        min: f64,
        max: f64,
        config: &WidgetConfig,
    ) -> bool {
        let changed = value != self.last_value;
        if !changed && config.refresh == 0 {
            return false;
        }
        self.last_value = value.to_string();

        let rect = config.rect;
        let fill_str = config.get_str("fill", "#00e600");
        let bg_str = config.get_str("outline", "#000000");
        let points = config.get_f64("points", 10.0) as usize;
        let horizontal = config.get_bool("horizontal", false);

        let fill_color = Color::from_hex(fill_str).unwrap_or(Color::new(0, 230, 0));
        let bg_color = Color::from_hex(bg_str).unwrap_or(Color::BLACK);

        canvas.fill_rect(rect, bg_color);

        let all_values: Vec<f64> = value
            .split(',')
            .filter_map(|s| s.trim().parse().ok())
            .collect();
        let data: &[f64] = if all_values.len() > points {
            &all_values[all_values.len() - points..]
        } else {
            &all_values
        };

        let n = data.len();
        if n == 0 {
            return changed;
        }

        let effective_min = min;
        let effective_max = max.max(effective_min + 1.0);
        let span = effective_max - effective_min;

        if !horizontal {
            let bar_w = ((rect.width as f64 / n as f64).floor() as u32).max(1);
            for (i, &v) in data.iter().enumerate() {
                let ratio = ((v - effective_min) / span).clamp(0.0, 1.0);
                let h = (ratio * rect.height as f64).round() as u32;
                if h == 0 {
                    continue;
                }
                let x = rect.x + (i as u32 * bar_w) as i32;
                let y = rect.y + (rect.height - h) as i32;
                canvas.fill_rect(
                    Rect {
                        x,
                        y,
                        width: bar_w.saturating_sub(1),
                        height: h,
                    },
                    fill_color,
                );
            }
        } else {
            let bar_h = ((rect.height as f64 / n as f64).floor() as u32).max(1);
            for (i, &v) in data.iter().enumerate() {
                let ratio = ((v - effective_min) / span).clamp(0.0, 1.0);
                let w = (ratio * rect.width as f64).round() as u32;
                if w == 0 {
                    continue;
                }
                let y = rect.y + (i as u32 * bar_h) as i32;
                canvas.fill_rect(
                    Rect {
                        x: rect.x,
                        y,
                        width: w,
                        height: bar_h.saturating_sub(1),
                    },
                    fill_color,
                );
            }
        }

        if config.debug_frame {
            canvas.stroke_rect(rect, Color::new(255, 0, 0), 1.0);
        }
        changed
    }
}
