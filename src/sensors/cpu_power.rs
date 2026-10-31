use std::fs;
use std::time::Instant;
use super::{apply_format, Sensor, SensorReading};
use super::history::History;

pub struct CpuPowerSensor {
    history: History,
    prev_energy_uj: u64,
    prev_time: Instant,
    max_watts: f64,
}

impl CpuPowerSensor {
    pub fn new(max_points: usize) -> Self {
        Self {
            history: History::new(max_points),
            prev_energy_uj: 0,
            prev_time: Instant::now(),
            max_watts: 28.0,
        }
    }

    fn read_total_energy_uj() -> u64 {
        let Ok(dir) = fs::read_dir("/sys/class/powercap") else {
            return 0;
        };
        let mut total = 0u64;
        for entry in dir.flatten() {
            let path = entry.path().join("energy_uj");
            if let Ok(s) = fs::read_to_string(&path) {
                total = total.saturating_add(s.trim().parse::<u64>().unwrap_or(0));
            }
        }
        total
    }
}

impl Sensor for CpuPowerSensor {
    fn name(&self) -> &str {
        "cpu_power"
    }

    fn sample(&mut self, _rate_ms: u64, format: &str) -> SensorReading {
        let now = Instant::now();
        let energy = Self::read_total_energy_uj();
        let elapsed = now.duration_since(self.prev_time).as_secs_f64();

        let watts = if elapsed > 0.0 && self.prev_energy_uj > 0 {
            let delta_uj = energy.saturating_sub(self.prev_energy_uj) as f64;
            (delta_uj / 1_000_000.0) / elapsed
        } else {
            0.0
        };

        self.prev_energy_uj = energy;
        self.prev_time = now;
        self.history.push(watts);

        let f0 = format!("{:.1}", watts);
        let f1 = self.history.join_csv();
        let value = apply_format(format, &[&f0, &f1]);
        SensorReading {
            value,
            min: 0.0,
            max: self.max_watts,
        }
    }
}
