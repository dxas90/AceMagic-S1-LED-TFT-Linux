pub mod history;
pub mod clock;
pub mod calendar;
pub mod cpu_usage;
pub mod cpu_temp;
pub mod cpu_power;
pub mod memory;
pub mod network;
pub mod space;
pub mod weather;

use std::collections::HashMap;

pub trait Sensor: Send + Sync {
    fn name(&self) -> &str;
    fn sample(&mut self, rate_ms: u64, format: &str) -> SensorReading;
    fn stop(&mut self) {}
}

pub struct SensorReading {
    pub value: String,
    pub min: f64,
    pub max: f64,
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
    use super::apply_format;

    #[test]
    fn test_apply_format() {
        assert_eq!(apply_format("{0}", &["hello"]), "hello");
        assert_eq!(apply_format("{0} {1}", &["42", "world"]), "42 world");
        assert_eq!(apply_format("{2}", &["a", "b", "c"]), "c");
        assert_eq!(apply_format("no tokens", &["x"]), "no tokens");
        assert_eq!(apply_format("{0}%", &["72"]), "72%");
    }
}
