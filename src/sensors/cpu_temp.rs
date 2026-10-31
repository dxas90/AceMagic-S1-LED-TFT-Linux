use super::history::History;
use super::{Sensor, SensorReading};
use std::fs;
use std::path::PathBuf;

pub struct CpuTempSensor {
    history: History,
    fahrenheit: bool,
    temp_path: Option<PathBuf>,
    min_temp: f64,
    max_temp: f64,
}

impl CpuTempSensor {
    pub fn new(max_points: usize, fahrenheit: bool) -> Self {
        let (min_temp, max_temp) = if fahrenheit {
            (70.0, 230.0)
        } else {
            (21.0, 105.0)
        };
        Self {
            history: History::new(max_points),
            fahrenheit,
            temp_path: None,
            min_temp,
            max_temp,
        }
    }

    fn find_temp_path() -> Option<PathBuf> {
        let dir = fs::read_dir("/sys/class/hwmon").ok()?;
        for entry in dir.flatten() {
            let base = entry.path();
            let name = fs::read_to_string(base.join("name")).unwrap_or_default();
            let driver = name.trim();

            // Priority labels per driver; first match wins.
            let labels: &[&str] = match driver {
                "coretemp" => &["Package id 0"],
                "k10temp" | "zenpower" => &["Tdie", "Tctl", "Tccd1"],
                _ => continue,
            };

            for i in 1..=32u32 {
                let label =
                    fs::read_to_string(base.join(format!("temp{i}_label"))).unwrap_or_default();
                let label_str = label.trim();
                if labels.contains(&label_str) {
                    let input_path = base.join(format!("temp{i}_input"));
                    if input_path.exists() {
                        return Some(input_path);
                    }
                }
            }
        }
        None
    }

    fn read_temp_celsius(&mut self) -> f64 {
        if self.temp_path.is_none() {
            self.temp_path = Self::find_temp_path();
        }
        let Some(ref path) = self.temp_path else {
            return 0.0;
        };
        let raw = fs::read_to_string(path).unwrap_or_default();
        raw.trim().parse::<f64>().unwrap_or(0.0) / 1000.0
    }

    pub fn celsius_to_fahrenheit(c: f64) -> f64 {
        c * 9.0 / 5.0 + 32.0
    }
}

impl Sensor for CpuTempSensor {
    fn name(&self) -> &str {
        "cpu_temp"
    }

    fn sample(&mut self, _rate_ms: u64) -> SensorReading {
        let celsius = self.read_temp_celsius();
        let temp = if self.fahrenheit {
            Self::celsius_to_fahrenheit(celsius)
        } else {
            celsius
        };
        self.history.push(temp);
        let unit = if self.fahrenheit { "F" } else { "C" };
        SensorReading::new(
            vec![
                format!("{:.1}", temp),  // {0} temperature value
                self.history.join_csv(), // {1} CSV history
                unit.to_string(),        // {2} unit (C or F)
            ],
            self.min_temp,
            self.max_temp,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn celsius_to_fahrenheit() {
        assert!((CpuTempSensor::celsius_to_fahrenheit(0.0) - 32.0).abs() < 0.01);
        assert!((CpuTempSensor::celsius_to_fahrenheit(100.0) - 212.0).abs() < 0.01);
        assert!((CpuTempSensor::celsius_to_fahrenheit(37.0) - 98.6).abs() < 0.1);
    }

    #[test]
    fn sample_returns_three_fields() {
        let mut s = CpuTempSensor::new(10, false);
        let r = s.sample(500);
        assert_eq!(r.fields.len(), 3);
        assert_eq!(r.fields[2], "C");
    }

    #[test]
    fn fahrenheit_unit_field() {
        let mut s = CpuTempSensor::new(10, true);
        let r = s.sample(500);
        assert_eq!(r.fields[2], "F");
        assert_eq!(r.min, 70.0);
        assert_eq!(r.max, 230.0);
    }
}
