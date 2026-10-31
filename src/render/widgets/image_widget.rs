use super::Widget;
use crate::config::{Color, WidgetConfig};
use crate::render::canvas::Canvas;
use std::collections::HashMap;

/// Image widget — displays a PNG or JPEG image.
/// `value` is the absolute file path to the image.
/// The image is loaded on first use and cached by path.
pub struct ImageWidget {
    cache: HashMap<String, image::DynamicImage>,
}

impl ImageWidget {
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
        }
    }
}

impl Widget for ImageWidget {
    fn name(&self) -> &str {
        "image"
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

        let rect = config.rect;

        if !self.cache.contains_key(value) {
            match image::open(value) {
                Ok(img) => {
                    self.cache.insert(value.to_string(), img);
                }
                Err(e) => {
                    tracing::warn!("ImageWidget: failed to load {value}: {e}");
                    return false;
                }
            }
        }

        let img = self.cache.get(value).unwrap();

        // Clear background first
        let bg = Color::from_hex(config.get_str("background", "#000000")).unwrap_or(Color::BLACK);
        canvas.fill_rect(rect, bg);

        canvas.draw_image_raw(img, rect.x, rect.y);

        if config.debug_frame {
            canvas.stroke_rect(rect, Color::new(255, 0, 0), 1.0);
        }
        true
    }
}
