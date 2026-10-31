use super::history::History;
use super::{Sensor, SensorReading};
use std::collections::HashMap;
use std::fs;

pub struct MemorySensor {
    usage_history: History,
    swap_history: History,
    free_history: History,
    cached_history: History,
    active_history: History,
    used_swap_history: History,
}

impl MemorySensor {
    pub fn new(max_points: usize) -> Self {
        Self {
            usage_history: History::new(max_points),
            swap_history: History::new(max_points),
            free_history: History::new(max_points),
            cached_history: History::new(max_points),
            active_history: History::new(max_points),
            used_swap_history: History::new(max_points),
        }
    }

    pub fn parse_meminfo(content: &str) -> HashMap<String, u64> {
        let mut map = HashMap::new();
        for line in content.lines() {
            if let Some((key, rest)) = line.split_once(':') {
                let kb: u64 = rest
                    .split_whitespace()
                    .next()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0);
                map.insert(key.trim().to_string(), kb * 1024);
            }
        }
        map
    }

    fn read_meminfo() -> HashMap<String, u64> {
        let Ok(content) = fs::read_to_string("/proc/meminfo") else {
            return HashMap::new();
        };
        Self::parse_meminfo(&content)
    }

    pub fn fmt_mib(bytes: u64) -> String {
        format!("{:.0}", bytes as f64 / (1024.0 * 1024.0))
    }

    pub fn fmt_bytes(bytes: u64) -> String {
        if bytes >= 1024 * 1024 * 1024 {
            format!("{:.1} GiB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
        } else if bytes >= 1024 * 1024 {
            format!("{:.0} MiB", bytes as f64 / (1024.0 * 1024.0))
        } else {
            format!("{:.0} KiB", bytes as f64 / 1024.0)
        }
    }
}

impl Sensor for MemorySensor {
    fn name(&self) -> &str {
        "memory"
    }

    fn sample(&mut self, _rate_ms: u64) -> SensorReading {
        let m = Self::read_meminfo();
        let get = |k: &str| *m.get(k).unwrap_or(&0);

        let total = get("MemTotal");
        let free = get("MemFree");
        let cached = get("Cached") + get("Buffers");
        let swap_total = get("SwapTotal");
        let swap_free = get("SwapFree");
        let used = total.saturating_sub(free);
        let active = used.saturating_sub(cached);
        let used_swap = swap_total.saturating_sub(swap_free);

        let pct = |n: u64, d: u64| {
            if d > 0 {
                n as f64 / d as f64 * 100.0
            } else {
                0.0
            }
        };
        let usage_pct = pct(active, total);
        let swap_pct = pct(used_swap, swap_total);
        let free_pct = pct(free, total);
        let cached_pct = pct(cached, total);

        self.usage_history.push(usage_pct);
        self.swap_history.push(swap_pct);
        self.free_history.push(free_pct);
        self.cached_history.push(cached_pct);
        self.active_history.push(usage_pct);
        self.used_swap_history.push(swap_pct);

        // Tokens matching memory.js (0-15):
        // {0} total  {1} swap_total  {2} usage%  {3} usage% csv
        // {4} swap%  {5} swap% csv   {6} free     {7} free% csv
        // {8} cached {9} cached% csv {10} active  {11} active% csv
        // {12} used_swap {13} used_swap% csv
        // {14} fmt active (e.g. "1.2 GiB")  {15} fmt used_swap
        SensorReading::new(
            vec![
                total.to_string(),
                swap_total.to_string(),
                format!("{:.1}", usage_pct),
                self.usage_history.join_csv(),
                format!("{:.1}", swap_pct),
                self.swap_history.join_csv(),
                free.to_string(),
                self.free_history.join_csv(),
                cached.to_string(),
                self.cached_history.join_csv(),
                active.to_string(),
                self.active_history.join_csv(),
                used_swap.to_string(),
                self.used_swap_history.join_csv(),
                Self::fmt_bytes(active),
                Self::fmt_bytes(used_swap),
            ],
            0.0,
            100.0,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = "\
MemTotal:       16348040 kB
MemFree:         8000000 kB
Buffers:          200000 kB
Cached:           400000 kB
SwapTotal:       4096000 kB
SwapFree:        4096000 kB
";

    #[test]
    fn parse_meminfo_basic() {
        let m = MemorySensor::parse_meminfo(FIXTURE);
        assert_eq!(m["MemTotal"], 16348040 * 1024);
        assert_eq!(m["MemFree"], 8000000 * 1024);
        assert_eq!(m["SwapFree"], 4096000 * 1024);
    }

    #[test]
    fn zero_swap_gives_zero_pct() {
        let fixture =
            "MemTotal: 8000000 kB\nMemFree: 4000000 kB\nSwapTotal: 0 kB\nSwapFree: 0 kB\n";
        let m = MemorySensor::parse_meminfo(fixture);
        let swap_total = *m.get("SwapTotal").unwrap_or(&0);
        let swap_free = *m.get("SwapFree").unwrap_or(&0);
        let used_swap = swap_total.saturating_sub(swap_free);
        let pct = if swap_total > 0 {
            used_swap as f64 / swap_total as f64 * 100.0
        } else {
            0.0
        };
        assert_eq!(pct, 0.0);
    }

    #[test]
    fn fmt_bytes_units() {
        assert!(MemorySensor::fmt_bytes(1024 * 1024 * 1024).contains("GiB"));
        assert!(MemorySensor::fmt_bytes(2 * 1024 * 1024).contains("MiB"));
        assert!(MemorySensor::fmt_bytes(512 * 1024).contains("KiB"));
    }

    #[test]
    fn sample_returns_sixteen_fields() {
        let mut s = MemorySensor::new(10);
        let r = s.sample(500);
        assert_eq!(r.fields.len(), 16);
    }
}
