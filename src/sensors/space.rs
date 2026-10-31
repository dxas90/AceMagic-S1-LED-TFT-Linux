use super::history::History;
use super::{Sensor, SensorReading};

pub struct SpaceSensor {
    mount_point: String,
    sensor_name: String,
    free_history: History,
    used_history: History,
}

impl SpaceSensor {
    pub fn new(name: impl Into<String>, mount_point: impl Into<String>, max_points: usize) -> Self {
        let n = name.into();
        let sensor_name = format!("space_{}", n);
        Self {
            sensor_name,
            mount_point: mount_point.into(),
            free_history: History::new(max_points),
            used_history: History::new(max_points),
        }
    }

    fn read_disk(mount: &str) -> (u64, u64, u64) {
        #[cfg(target_os = "linux")]
        if let Ok(stat) = nix::sys::statvfs::statvfs(mount) {
            let bsize = stat.block_size();
            let total = stat.blocks() * bsize;
            let free = stat.blocks_free() * bsize;
            return (total, free, total.saturating_sub(free));
        }
        let _ = mount;
        (0, 0, 0)
    }

    pub fn fmt_mib(bytes: u64) -> String {
        format!("{:.0}", bytes as f64 / (1024.0 * 1024.0))
    }

    pub fn pct(n: u64, d: u64) -> f64 {
        if d > 0 {
            n as f64 / d as f64 * 100.0
        } else {
            0.0
        }
    }
}

impl Sensor for SpaceSensor {
    fn name(&self) -> &str {
        &self.sensor_name
    }

    fn sample(&mut self, _rate_ms: u64) -> SensorReading {
        let (total, free, used) = Self::read_disk(&self.mount_point);
        let free_pct = Self::pct(free, total);
        let used_pct = Self::pct(used, total);

        self.free_history.push(free_pct);
        self.used_history.push(used_pct);

        // Tokens matching space.js (0-8):
        // {0} mount  {1} free% csv  {2} used% csv
        // {3} free%  {4} used%      {5} free MiB
        // {6} used MiB  {7} total MiB  {8} "MiB"
        SensorReading::new(
            vec![
                self.mount_point.clone(),
                self.free_history.join_csv(),
                self.used_history.join_csv(),
                format!("{:.1}", free_pct),
                format!("{:.1}", used_pct),
                Self::fmt_mib(free),
                Self::fmt_mib(used),
                Self::fmt_mib(total),
                "MiB".to_string(),
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
    fn pct_basic() {
        assert!((SpaceSensor::pct(50, 100) - 50.0).abs() < 0.01);
        assert_eq!(SpaceSensor::pct(0, 0), 0.0);
    }

    #[test]
    fn fmt_mib_converts() {
        // 1 MiB = 1,048,576 bytes → "1"
        assert_eq!(SpaceSensor::fmt_mib(1024 * 1024), "1");
        assert_eq!(SpaceSensor::fmt_mib(0), "0");
    }

    #[test]
    fn sample_returns_nine_fields() {
        let mut s = SpaceSensor::new("root", "/", 10);
        let r = s.sample(500);
        assert_eq!(r.fields.len(), 9);
        assert_eq!(r.fields[0], "/");
        assert_eq!(r.fields[8], "MiB");
    }
}
