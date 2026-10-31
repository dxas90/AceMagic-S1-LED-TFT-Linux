use std::fs;
use std::path::PathBuf;
use super::{apply_format, Sensor, SensorReading};
use super::history::History;

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
            if !name.trim().starts_with("coretemp") {
                continue;
            }
            // Walk tempN_label looking for "Package id 0"
            for i in 1..=32u32 {
                let label = fs::read_to_string(base.join(format!("temp{i}_label")))
                    .unwrap_or_default();
                if label.trim() == "Package id 0" {
                    return Some(base.join(format!("temp{i}_input")));
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
        let millideg: f64 = raw.trim().parse().unwrap_or(0.0);
        millideg / 1000.0
    }
}

impl Sensor for CpuTempSensor {
    fn name(&self) -> &str {
        "cpu_temp"
    }

    fn sample(&mut self, _rate_ms: u64, format: &str) -> SensorReading {
        let celsius = self.read_temp_celsius();
        let temp = if self.fahrenheit {
            celsius * 9.0 / 5.0 + 32.0
        } else {
            celsius
        };
        self.history.push(temp);
        let unit = if self.fahrenheit { "F" } else { "C" };
        let f0 = format!("{:.1}", temp);
        let f1 = self.history.join_csv();
        let value = apply_format(format, &[&f0, &f1, unit]);
        SensorReading {
            value,
            min: self.min_temp,
            max: self.max_temp,
        }
    }
}
