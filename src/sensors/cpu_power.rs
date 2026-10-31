use super::history::History;
use super::{Sensor, SensorReading};
use std::fs;
use std::time::Instant;

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

    pub fn compute_watts(delta_uj: u64, elapsed_secs: f64) -> f64 {
        if elapsed_secs <= 0.0 {
            return 0.0;
        }
        delta_uj as f64 / 1_000_000.0 / elapsed_secs
    }

    fn read_total_energy_uj() -> u64 {
        let Ok(dir) = fs::read_dir("/sys/class/powercap") else {
            return 0;
        };
        let mut total = 0u64;
        for entry in dir.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            // Only include package-level entries (intel-rapl:N), not sub-domains
            // (intel-rapl:N:M) which would double-count package energy.
            if name_str.starts_with("intel-rapl:") && name_str.matches(':').count() == 1 {
                let path = entry.path().join("energy_uj");
                if let Ok(s) = fs::read_to_string(&path) {
                    total = total.saturating_add(s.trim().parse::<u64>().unwrap_or(0));
                }
            }
        }
        total
    }
}

impl Sensor for CpuPowerSensor {
    fn name(&self) -> &str {
        "cpu_power"
    }

    fn sample(&mut self, _rate_ms: u64) -> SensorReading {
        let now = Instant::now();
        let energy = Self::read_total_energy_uj();
        let elapsed = now.duration_since(self.prev_time).as_secs_f64();

        let watts = if self.prev_energy_uj > 0 {
            Self::compute_watts(energy.saturating_sub(self.prev_energy_uj), elapsed)
        } else {
            0.0
        };

        self.prev_energy_uj = energy;
        self.prev_time = now;
        self.history.push(watts);

        SensorReading::new(
            vec![
                format!("{:.1}", watts), // {0} watts
                self.history.join_csv(), // {1} CSV history
            ],
            0.0,
            self.max_watts,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compute_watts_basic() {
        // 1 million µJ over 1 second = 1 W
        assert!((CpuPowerSensor::compute_watts(1_000_000, 1.0) - 1.0).abs() < 0.001);
    }

    #[test]
    fn compute_watts_zero_elapsed() {
        assert_eq!(CpuPowerSensor::compute_watts(1_000_000, 0.0), 0.0);
    }

    #[test]
    fn sample_returns_two_fields() {
        let mut s = CpuPowerSensor::new(10);
        let r = s.sample(500);
        assert_eq!(r.fields.len(), 2);
        assert_eq!(r.min, 0.0);
    }
}
