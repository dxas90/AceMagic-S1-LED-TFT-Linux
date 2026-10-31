use super::Widget;
use crate::config::{Color, WidgetConfig};
use crate::render::canvas::Canvas;

pub struct TextWidget {
    last_value: String,
}

impl TextWidget {
    pub fn new() -> Self { Self { last_value: String::new() } }
}

impl Widget for TextWidget {
    fn name(&self) -> &str { "text" }

    fn draw(&mut self, canvas: &mut Canvas, value: &str, _min: f64, _max: f64, config: &WidgetConfig) -> bool {
        let changed = value != self.last_value;
        if !changed && config.refresh == 0 { return false; }
        self.last_value = value.to_string();

        let font_str  = config.get_str("font",       "18px Arial");
        let color_str = config.get_str("color",      "#ffffff");
        let align     = config.get_str("align",      "left");
        let bg_str    = config.get_str("background", "#000000");

        let color = Color::from_hex(color_str).unwrap_or(Color::WHITE);
        let bg    = Color::from_hex(bg_str).unwrap_or(Color::BLACK);
        let (size, font_name) = parse_css_font(font_str);
        let rect = config.rect;

        canvas.fill_rect(rect, bg);

        // Vertically center: top = rect.y + (height - size) / 2
        let text_y = rect.y as f32 + (rect.height as f32 - size) / 2.0;
        let text_x = match align {
            "center" => rect.x as f32 + rect.width as f32 / 2.0,
            "right"  => rect.x as f32 + rect.width as f32,
            _        => rect.x as f32,
        };

        canvas.draw_text(value, text_x, text_y, &font_name, size, color, align);

        if config.debug_frame {
            canvas.stroke_rect(rect, Color::new(255, 0, 0), 1.0);
        }
        changed
    }
}

/// Parse a CSS font string like "bold 18px Arial" or "18px sans-serif".
/// Returns (pixel_size, family_name).
pub fn parse_css_font(font: &str) -> (f32, String) {
    let parts: Vec<&str> = font.split_whitespace().collect();
    for (i, part) in parts.iter().enumerate() {
        if part.ends_with("px") {
            let size = part.trim_end_matches("px").parse::<f32>().unwrap_or(16.0);
            let family = parts[i + 1..].join(" ");
            return (size, if family.is_empty() { "default".to_string() } else { family });
        }
        // Handle "NNpt" as well
        if part.ends_with("pt") {
            let pts = part.trim_end_matches("pt").parse::<f32>().unwrap_or(12.0);
            let size = pts * 1.333;
            let family = parts[i + 1..].join(" ");
            return (size, if family.is_empty() { "default".to_string() } else { family });
        }
    }
    (16.0, "default".to_string())
}
