use super::{iconify::render_svg_into_canvas, Widget};
use crate::config::{Color, WidgetConfig};
use crate::render::canvas::Canvas;

/// Renders a weather/temperature/wind-direction icon using the `wi` Iconify set.
/// Config:
///   "type"  — "weather" | "temperature" | "winddirection" (default "weather")
///   "color" — fill color (default "#ffffff")
///   "day"   — bool, use day variant of icons (default true)
/// `value` interpretation:
///   "weather"       — WMO weather code (integer string)
///   "temperature"   — temperature in °C (used to pick cold/warm icon)
///   "winddirection" — wind direction in degrees (0=N, 90=E, ...)
pub struct WeatherIconWidget {
    last_value: String,
    last_svg: Option<String>,
}

impl WeatherIconWidget {
    pub fn new() -> Self {
        Self {
            last_value: String::new(),
            last_svg: None,
        }
    }
}

impl Widget for WeatherIconWidget {
    fn name(&self) -> &str {
        "weather_icon"
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

        let icon_type = config.get_str("type", "weather");
        let color_str = config.get_str("color", "#ffffff");
        let is_day = config.get_bool("day", true);
        let rect = config.rect;

        let icon_name = match icon_type {
            "temperature" => icon_for_temperature(value),
            "winddirection" => icon_for_wind_direction(value),
            _ => icon_for_wmo_code(value, is_day),
        };

        let changed = value != self.last_value;
        if !changed && self.last_svg.is_some() && config.refresh == 0 {
            // Re-draw cached SVG without refetching
            if let Some(svg) = &self.last_svg.clone() {
                canvas.fill_rect(
                    rect,
                    Color::from_hex(config.get_str("background", "#000000"))
                        .unwrap_or(Color::BLACK),
                );
                render_svg_into_canvas(svg, canvas, rect);
            }
            return false;
        }
        self.last_value = value.to_string();

        // Build the full Iconify SVG URL path: wi/{icon_name}
        let cache_dir = std::env::var("HOME")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| std::path::PathBuf::from("/tmp"))
            .join(".cache")
            .join("acemagic-s1")
            .join("icons")
            .join("wi");
        let cache_path = cache_dir.join(format!("{}.svg", icon_name));

        let raw_svg = if cache_path.exists() {
            std::fs::read_to_string(&cache_path).ok()
        } else {
            let url = format!("https://api.iconify.design/wi/{}.svg", icon_name);
            let fetched = fetch_with_curl(&url);
            if let Some(ref s) = fetched {
                let _ = std::fs::create_dir_all(&cache_dir);
                let _ = std::fs::write(&cache_path, s);
            }
            fetched
        };

        let Some(raw) = raw_svg else { return false };
        let svg = raw.replace("currentColor", color_str);
        self.last_svg = Some(svg.clone());

        let bg = Color::from_hex(config.get_str("background", "#000000")).unwrap_or(Color::BLACK);
        canvas.fill_rect(rect, bg);
        render_svg_into_canvas(&svg, canvas, rect);

        if config.debug_frame {
            canvas.stroke_rect(rect, Color::new(255, 0, 0), 1.0);
        }
        changed
    }
}

fn icon_for_wmo_code(value: &str, is_day: bool) -> &'static str {
    let code: u32 = value.trim().parse().unwrap_or(0);
    if is_day {
        match code {
            0 => "day-sunny",
            1 | 2 => "day-cloudy",
            3 => "cloudy",
            45 | 48 => "fog",
            51..=55 => "sprinkle",
            56 | 57 => "sleet",
            61..=65 => "rain",
            66 | 67 => "rain-mix",
            71..=75 => "snow",
            77 => "snow",
            80..=82 => "showers",
            85 | 86 => "snow",
            95 => "thunderstorm",
            96 | 99 => "storm-showers",
            _ => "na",
        }
    } else {
        match code {
            0 => "night-clear",
            1 | 2 => "night-alt-cloudy",
            3 => "cloudy",
            45 | 48 => "night-fog",
            51..=55 => "night-alt-sprinkle",
            61..=65 => "night-alt-rain",
            71..=75 => "night-alt-snow",
            95 => "night-alt-thunderstorm",
            96 | 99 => "night-alt-storm-showers",
            _ => "na",
        }
    }
}

fn icon_for_temperature(value: &str) -> &'static str {
    let t: f64 = value.trim().parse().unwrap_or(20.0);
    if t < 0.0 {
        "thermometer-exterior"
    } else if t < 10.0 {
        "thermometer-low"
    } else if t < 25.0 {
        "thermometer"
    } else if t < 35.0 {
        "thermometer-high"
    } else {
        "hot"
    }
}

fn icon_for_wind_direction(value: &str) -> &'static str {
    let deg: f64 = value.trim().parse().unwrap_or(0.0);
    let d = ((deg % 360.0 + 360.0) % 360.0) as u32;
    // 8 compass directions, each covering 45°
    match d {
        0..=22 | 338..=360 => "wind-direction-n",
        23..=67 => "wind-direction-ne",
        68..=112 => "wind-direction-e",
        113..=157 => "wind-direction-se",
        158..=202 => "wind-direction-s",
        203..=247 => "wind-direction-sw",
        248..=292 => "wind-direction-w",
        293..=337 => "wind-direction-nw",
        _ => "wind-direction-n",
    }
}

fn fetch_with_curl(url: &str) -> Option<String> {
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
