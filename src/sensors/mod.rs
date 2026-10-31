pub mod calendar;
pub mod clock;
pub mod cpu_power;
pub mod cpu_temp;
pub mod cpu_usage;
pub mod history;
pub mod memory;
pub mod network;
pub mod space;
pub mod weather;

use std::collections::HashMap;

pub trait Sensor: Send + Sync {
    fn name(&self) -> &str;
    /// Sample the sensor and return raw data fields. Field indices match the
    /// {N} tokens used in widget format strings. The first field ({0}) is
    /// always the primary display value.
    fn sample(&mut self, rate_ms: u64) -> SensorReading;
    fn stop(&mut self) {}
}

pub struct SensorReading {
    /// Raw data fields indexed by {N} tokens in format strings.
    pub fields: Vec<String>,
    pub min: f64,
    pub max: f64,
}

impl SensorReading {
    pub fn new(fields: Vec<String>, min: f64, max: f64) -> Self {
        Self { fields, min, max }
    }

    /// Apply a widget format string using this reading's fields.
    /// An empty template returns the first field (primary value).
    pub fn format(&self, template: &str) -> String {
        let refs: Vec<&str> = self.fields.iter().map(|s| s.as_str()).collect();
        apply_format(template, &refs)
    }

    /// The primary display value — always `fields[0]`.
    pub fn value(&self) -> &str {
        self.fields.first().map(|s| s.as_str()).unwrap_or("")
    }
}

pub type SensorMap = HashMap<String, Box<dyn Sensor>>;

/// Substitutes {N} tokens in a format string with the corresponding field values.
/// Example: apply_format("{0} °{2}", &["42", "cpu", "C"]) → "42 °C"
pub fn apply_format(template: &str, fields: &[&str]) -> String {
    if template.is_empty() {
        return fields.first().copied().unwrap_or("").to_string();
    }
    let mut out = String::with_capacity(template.len() + 16);
    let bytes = template.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'{' {
            // try to parse a number up to the next '}'
            let start = i + 1;
            let mut j = start;
            while j < bytes.len() && bytes[j] != b'}' {
                j += 1;
            }
            if j < bytes.len() {
                // found closing brace
                let num_str = &template[start..j];
                if let Ok(idx) = num_str.parse::<usize>() {
                    if let Some(field) = fields.get(idx) {
                        out.push_str(field);
                        i = j + 1;
                        continue;
                    }
                }
                // not a valid token — emit literally
                out.push('{');
                out.push_str(num_str);
                out.push('}');
                i = j + 1;
            } else {
                // no closing brace found — emit rest literally
                out.push_str(&template[i..]);
                break;
            }
        } else {
            out.push(bytes[i] as char);
            i += 1;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{apply_format, SensorReading};

    #[test]
    fn test_apply_format() {
        assert_eq!(apply_format("{0}", &["hello"]), "hello");
        assert_eq!(apply_format("{0} {1}", &["42", "world"]), "42 world");
        assert_eq!(apply_format("{2}", &["a", "b", "c"]), "c");
        assert_eq!(apply_format("no tokens", &["x"]), "no tokens");
        assert_eq!(apply_format("{0}%", &["72"]), "72%");
    }

    #[test]
    fn apply_format_empty_template_returns_first_field() {
        assert_eq!(apply_format("", &["primary", "secondary"]), "primary");
    }

    #[test]
    fn apply_format_out_of_range_index_emits_literal() {
        assert_eq!(apply_format("{5}", &["only_one"]), "{5}");
    }

    #[test]
    fn sensor_reading_value_returns_first_field() {
        let r = SensorReading::new(vec!["42".into(), "history".into()], 0.0, 100.0);
        assert_eq!(r.value(), "42");
    }

    #[test]
    fn sensor_reading_value_empty_fields() {
        let r = SensorReading::new(vec![], 0.0, 100.0);
        assert_eq!(r.value(), "");
    }

    #[test]
    fn sensor_reading_format_applies_template() {
        let r = SensorReading::new(vec!["72".into(), "°C".into()], 0.0, 100.0);
        assert_eq!(r.format("{0} {1}"), "72 °C");
    }

    #[test]
    fn sensor_reading_format_empty_returns_first_field() {
        let r = SensorReading::new(vec!["42".into(), "other".into()], 0.0, 100.0);
        assert_eq!(r.format(""), "42");
    }

    #[test]
    fn sensor_reading_min_max() {
        let r = SensorReading::new(vec!["50".into()], 10.0, 90.0);
        assert_eq!(r.min, 10.0);
        assert_eq!(r.max, 90.0);
    }
}
