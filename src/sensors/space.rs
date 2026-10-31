use super::{apply_format, Sensor, SensorReading};
use super::history::History;

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
        {
            if let Ok(stat) = nix::sys::statvfs::statvfs(mount) {
                let bsize = stat.block_size() as u64;
                let total = stat.blocks()      * bsize;
                let free  = stat.blocks_free() * bsize;
                let used  = total.saturating_sub(free);
                return (total, free, used);
            }
        }
        let _ = mount;
        (0, 0, 0)
    }

    fn fmt_mib(bytes: u64) -> String {
        format!("{:.0}", bytes as f64 / (1024.0 * 1024.0))
    }
}

impl Sensor for SpaceSensor {
    fn name(&self) -> &str {
        &self.sensor_name
    }

    fn sample(&mut self, _rate_ms: u64, format: &str) -> SensorReading {
        let (total, free, used) = Self::read_disk(&self.mount_point);

        let free_pct = if total > 0 { free as f64 / total as f64 * 100.0 } else { 0.0 };
        let used_pct = if total > 0 { used as f64 / total as f64 * 100.0 } else { 0.0 };

        self.free_history.push(free_pct);
        self.used_history.push(used_pct);

        // Tokens matching space.js (0-8):
        // {0} mount point  {1} free% history  {2} used% history
        // {3} last free%   {4} last used%
        // {5} free MiB     {6} used MiB        {7} total MiB   {8} "MiB"
        let fields: [String; 9] = [
            self.mount_point.clone(),
            self.free_history.join_csv(),
            self.used_history.join_csv(),
            format!("{:.1}", free_pct),
            format!("{:.1}", used_pct),
            Self::fmt_mib(free),
            Self::fmt_mib(used),
            Self::fmt_mib(total),
            "MiB".to_string(),
        ];
        let refs: Vec<&str> = fields.iter().map(|s| s.as_str()).collect();
        let value = apply_format(format, &refs);
        SensorReading { value, min: 0.0, max: 100.0 }
    }
}
