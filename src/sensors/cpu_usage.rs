use std::fs;
use super::{apply_format, Sensor, SensorReading};
use super::history::History;

pub struct CpuUsageSensor {
    history: History,
    prev_total: u64,
    prev_busy: u64,
}

impl CpuUsageSensor {
    pub fn new(max_points: usize) -> Self {
        Self {
            history: History::new(max_points),
            prev_total: 0,
            prev_busy: 0,
        }
    }

    fn read_proc_stat() -> Option<(u64, u64)> {
        let content = fs::read_to_string("/proc/stat").ok()?;
        let line = content.lines().find(|l| l.starts_with("cpu "))?;
        // Fields: user nice system idle iowait irq softirq steal guest guest_nice
        let fields: Vec<u64> = line
            .split_whitespace()
            .skip(1)
            .take(10)
            .filter_map(|s| s.parse().ok())
            .collect();
        if fields.len() < 4 {
            return None;
        }
        let idle = fields[3] + fields.get(4).copied().unwrap_or(0);
        let total: u64 = fields.iter().sum();
        let busy = total.saturating_sub(idle);
        Some((total, busy))
    }
}

impl Sensor for CpuUsageSensor {
    fn name(&self) -> &str {
        "cpu_usage"
    }

    fn sample(&mut self, _rate_ms: u64, format: &str) -> SensorReading {
        let usage = if let Some((total, busy)) = Self::read_proc_stat() {
            let d_total = total.saturating_sub(self.prev_total);
            let d_busy = busy.saturating_sub(self.prev_busy);
            self.prev_total = total;
            self.prev_busy = busy;
            if d_total == 0 {
                0.0
            } else {
                (d_busy as f64 / d_total as f64) * 100.0
            }
        } else {
            0.0
        };

        self.history.push(usage);
        let f0 = format!("{:.0}", usage);
        let f1 = self.history.join_csv();
        let value = apply_format(format, &[&f0, &f1]);
        SensorReading { value, min: 0.0, max: 100.0 }
    }
}
