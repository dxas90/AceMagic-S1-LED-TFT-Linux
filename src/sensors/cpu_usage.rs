use super::history::History;
use super::{Sensor, SensorReading};
use std::fs;

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

    pub fn parse_stat_line(line: &str) -> Option<(u64, u64)> {
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
        Some((total, total.saturating_sub(idle)))
    }

    fn read_proc_stat() -> Option<(u64, u64)> {
        let content = fs::read_to_string("/proc/stat").ok()?;
        let line = content.lines().find(|l| l.starts_with("cpu "))?;
        Self::parse_stat_line(line)
    }
}

impl Sensor for CpuUsageSensor {
    fn name(&self) -> &str {
        "cpu_usage"
    }

    fn sample(&mut self, _rate_ms: u64) -> SensorReading {
        let usage = if let Some((total, busy)) = Self::read_proc_stat() {
            let dt = total.saturating_sub(self.prev_total);
            let db = busy.saturating_sub(self.prev_busy);
            self.prev_total = total;
            self.prev_busy = busy;
            if dt == 0 {
                0.0
            } else {
                db as f64 / dt as f64 * 100.0
            }
        } else {
            0.0
        };

        self.history.push(usage);
        SensorReading::new(
            vec![
                format!("{:.0}", usage), // {0} integer percent
                self.history.join_csv(), // {1} CSV history
            ],
            0.0,
            100.0,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_stat_line_basic() {
        // user nice system idle iowait irq softirq steal guest guest_nice
        let line = "cpu  100 0 20 380 0 0 0 0 0 0";
        let (total, busy) = CpuUsageSensor::parse_stat_line(line).unwrap();
        // total = 500, idle = 380, busy = 120
        assert_eq!(total, 500);
        assert_eq!(busy, 120);
    }

    #[test]
    fn parse_stat_with_iowait() {
        let line = "cpu  100 0 20 300 80 0 0 0 0 0";
        let (total, busy) = CpuUsageSensor::parse_stat_line(line).unwrap();
        // idle = 300 + 80 = 380, total = 500, busy = 120
        assert_eq!(total, 500);
        assert_eq!(busy, 120);
    }

    #[test]
    fn delta_computation() {
        let mut s = CpuUsageSensor::new(10);
        // First sample sets baseline
        s.prev_total = 1000;
        s.prev_busy = 200;
        // Simulate: 100 total ticks elapsed, 50 busy
        let (total, busy) = (1100u64, 250u64);
        let dt = total - s.prev_total; // 100
        let db = busy - s.prev_busy; // 50
        let pct = db as f64 / dt as f64 * 100.0;
        assert!((pct - 50.0).abs() < 0.01);
    }

    #[test]
    fn sample_returns_two_fields() {
        let mut s = CpuUsageSensor::new(10);
        let r = s.sample(500);
        assert_eq!(r.fields.len(), 2);
        assert_eq!(r.min, 0.0);
        assert_eq!(r.max, 100.0);
    }
}
