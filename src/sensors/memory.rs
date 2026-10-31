use std::collections::HashMap;
use std::fs;
use super::{apply_format, Sensor, SensorReading};
use super::history::History;

pub struct MemorySensor {
    usage_history:      History,
    swap_history:       History,
    free_history:       History,
    cached_history:     History,
    active_history:     History,
    used_swap_history:  History,
}

impl MemorySensor {
    pub fn new(max_points: usize) -> Self {
        Self {
            usage_history:     History::new(max_points),
            swap_history:      History::new(max_points),
            free_history:      History::new(max_points),
            cached_history:    History::new(max_points),
            active_history:    History::new(max_points),
            used_swap_history: History::new(max_points),
        }
    }

    fn read_meminfo() -> HashMap<String, u64> {
        let mut map = HashMap::new();
        let Ok(content) = fs::read_to_string("/proc/meminfo") else {
            return map;
        };
        for line in content.lines() {
            if let Some((key, rest)) = line.split_once(':') {
                let kb: u64 = rest
                    .split_whitespace()
                    .next()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0);
                map.insert(key.trim().to_string(), kb * 1024); // convert kB → bytes
            }
        }
        map
    }

    fn fmt_mib(bytes: u64) -> String {
        format!("{:.0}", bytes as f64 / (1024.0 * 1024.0))
    }

    fn fmt_bytes(bytes: u64) -> String {
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

    fn sample(&mut self, _rate_ms: u64, format: &str) -> SensorReading {
        let m = Self::read_meminfo();

        let total      = *m.get("MemTotal").unwrap_or(&0);
        let free       = *m.get("MemFree").unwrap_or(&0);
        let cached     = *m.get("Cached").unwrap_or(&0)
                       + *m.get("Buffers").unwrap_or(&0);
        let swap_total = *m.get("SwapTotal").unwrap_or(&0);
        let swap_free  = *m.get("SwapFree").unwrap_or(&0);

        let used       = total.saturating_sub(free);
        let active     = used.saturating_sub(cached);
        let used_swap  = swap_total.saturating_sub(swap_free);

        let usage_pct      = if total > 0 { active  as f64 / total      as f64 * 100.0 } else { 0.0 };
        let swap_pct       = if swap_total > 0 { used_swap as f64 / swap_total as f64 * 100.0 } else { 0.0 };
        let free_pct       = if total > 0 { free    as f64 / total      as f64 * 100.0 } else { 0.0 };
        let cached_pct     = if total > 0 { cached  as f64 / total      as f64 * 100.0 } else { 0.0 };
        let active_pct     = usage_pct;
        let used_swap_pct  = swap_pct;

        self.usage_history.push(usage_pct);
        self.swap_history.push(swap_pct);
        self.free_history.push(free_pct);
        self.cached_history.push(cached_pct);
        self.active_history.push(active_pct);
        self.used_swap_history.push(used_swap_pct);

        // Tokens matching memory.js (0-15):
        // {0}  total bytes
        // {1}  swap_total bytes
        // {2}  usage%
        // {3}  usage% history csv
        // {4}  swap%
        // {5}  swap% history csv
        // {6}  free bytes
        // {7}  free% history csv
        // {8}  cached bytes
        // {9}  cached% history csv
        // {10} active bytes
        // {11} active% history csv
        // {12} used_swap bytes
        // {13} used_swap% history csv
        // {14} formatted active (e.g. "1.2 GiB")
        // {15} formatted used_swap
        let fields: [String; 16] = [
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
        ];
        let refs: Vec<&str> = fields.iter().map(|s| s.as_str()).collect();
        let value = apply_format(format, &refs);
        SensorReading { value, min: 0.0, max: 100.0 }
    }
}
