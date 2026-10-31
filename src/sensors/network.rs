use std::fs;
use std::time::Instant;
use super::{apply_format, Sensor, SensorReading};
use super::history::History;

pub struct NetworkSensor {
    interface: String,
    sensor_name: String,
    rx_history: History,
    tx_history: History,
    rx_pps_history: History,
    tx_pps_history: History,
    prev_rx_bytes:   u64,
    prev_tx_bytes:   u64,
    prev_rx_packets: u64,
    prev_tx_packets: u64,
    prev_time: Instant,
    max_bps: f64,
}

impl NetworkSensor {
    pub fn new(interface: impl Into<String>, max_points: usize) -> Self {
        let iface = interface.into();
        let sensor_name = format!("network_{}", iface);
        Self {
            interface: iface,
            sensor_name,
            rx_history:  History::new(max_points),
            tx_history:  History::new(max_points),
            rx_pps_history: History::new(max_points),
            tx_pps_history: History::new(max_points),
            prev_rx_bytes:   0,
            prev_tx_bytes:   0,
            prev_rx_packets: 0,
            prev_tx_packets: 0,
            prev_time: Instant::now(),
            max_bps: 0.0,
        }
    }

    fn read_net_dev(&self) -> Option<(u64, u64, u64, u64)> {
        let content = fs::read_to_string("/proc/net/dev").ok()?;
        for line in content.lines() {
            let trimmed = line.trim_start();
            if trimmed.starts_with(&self.interface) {
                let after_colon = trimmed.splitn(2, ':').nth(1)?;
                let fields: Vec<u64> = after_colon
                    .split_whitespace()
                    .filter_map(|s| s.parse().ok())
                    .collect();
                // /proc/net/dev fields: rx_bytes rx_packets rx_errs rx_drop ... tx_bytes tx_packets ...
                // rx: fields[0]=bytes, [1]=packets; tx: fields[8]=bytes, [9]=packets
                let rx_bytes   = fields.get(0).copied().unwrap_or(0);
                let rx_packets = fields.get(1).copied().unwrap_or(0);
                let tx_bytes   = fields.get(8).copied().unwrap_or(0);
                let tx_packets = fields.get(9).copied().unwrap_or(0);
                return Some((rx_bytes, rx_packets, tx_bytes, tx_packets));
            }
        }
        None
    }

    fn read_link_speed(&self) -> u64 {
        let path = format!("/sys/class/net/{}/speed", self.interface);
        fs::read_to_string(&path)
            .ok()
            .and_then(|s| s.trim().parse::<u64>().ok())
            .map(|mbps| mbps * 1_000_000 / 8)  // Mbit/s → bytes/s
            .unwrap_or(0)
    }

    fn read_mtu(&self) -> u64 {
        let path = format!("/sys/class/net/{}/mtu", self.interface);
        fs::read_to_string(&path)
            .ok()
            .and_then(|s| s.trim().parse().ok())
            .unwrap_or(1500)
    }

    fn fmt_rate(bps: f64) -> String {
        if bps >= 1_000_000_000.0 {
            format!("{:.2} Gbps", bps / 1_000_000_000.0)
        } else if bps >= 1_000_000.0 {
            format!("{:.1} Mbps", bps / 1_000_000.0)
        } else if bps >= 1_000.0 {
            format!("{:.0} Kbps", bps / 1_000.0)
        } else {
            format!("{:.0} bps", bps)
        }
    }
}

impl Sensor for NetworkSensor {
    fn name(&self) -> &str {
        &self.sensor_name
    }

    fn sample(&mut self, _rate_ms: u64, format: &str) -> SensorReading {
        let now = Instant::now();
        let elapsed = now.duration_since(self.prev_time).as_secs_f64().max(0.001);

        let (rx_bytes, rx_packets, tx_bytes, tx_packets) =
            self.read_net_dev().unwrap_or((0, 0, 0, 0));

        let rx_bps = (rx_bytes.saturating_sub(self.prev_rx_bytes)     as f64) / elapsed;
        let tx_bps = (tx_bytes.saturating_sub(self.prev_tx_bytes)     as f64) / elapsed;
        let rx_pps = (rx_packets.saturating_sub(self.prev_rx_packets) as f64) / elapsed;
        let tx_pps = (tx_packets.saturating_sub(self.prev_tx_packets) as f64) / elapsed;

        // Auto-scale max_bps to 1.5× the max observed, capped by link speed
        let link_cap = self.read_link_speed() as f64;
        let observed = rx_bps.max(tx_bps);
        self.max_bps = (self.max_bps.max(observed * 1.5))
            .min(if link_cap > 0.0 { link_cap } else { 1_000_000_000.0 });

        self.prev_rx_bytes   = rx_bytes;
        self.prev_tx_bytes   = tx_bytes;
        self.prev_rx_packets = rx_packets;
        self.prev_tx_packets = tx_packets;
        self.prev_time = now;

        self.rx_history.push(rx_bps);
        self.tx_history.push(tx_bps);
        self.rx_pps_history.push(rx_pps);
        self.tx_pps_history.push(tx_pps);

        let mtu = self.read_mtu();

        // Tokens matching network.js (0-18):
        // {0}  interface name
        // {1}  rx_bytes last sample
        // {2}  rx_bytes history csv
        // {3}  rx_rate formatted (e.g. "12.3 Mbps")
        // {4}  tx_bytes last sample
        // {5}  tx_bytes history csv
        // {6}  tx_rate formatted
        // {7}  rx_packets last
        // {8}  rx_pps
        // {9}  rx_packets history csv
        // {10} tx_packets last
        // {11} tx_pps
        // {12} tx_packets history csv
        // {13} link_speed Mbit/s
        // {14} mtu
        // {15} rx bits/s raw
        // {16} tx bits/s raw
        // {17} ipv4 (not read, placeholder)
        // {18} ipv6 (not read, placeholder)
        let link_mbps = (self.read_link_speed() * 8 / 1_000_000).to_string();
        let fields: [String; 19] = [
            self.interface.clone(),
            rx_bytes.to_string(),
            self.rx_history.join_csv(),
            Self::fmt_rate(rx_bps * 8.0),
            tx_bytes.to_string(),
            self.tx_history.join_csv(),
            Self::fmt_rate(tx_bps * 8.0),
            rx_packets.to_string(),
            format!("{:.0}", rx_pps),
            self.rx_pps_history.join_csv(),
            tx_packets.to_string(),
            format!("{:.0}", tx_pps),
            self.tx_pps_history.join_csv(),
            link_mbps,
            mtu.to_string(),
            format!("{:.0}", rx_bps * 8.0),
            format!("{:.0}", tx_bps * 8.0),
            String::new(), // ipv4 — would need netlink; skip for now
            String::new(), // ipv6
        ];
        let refs: Vec<&str> = fields.iter().map(|s| s.as_str()).collect();
        let value = apply_format(format, &refs);
        SensorReading { value, min: 0.0, max: self.max_bps * 8.0 }
    }
}
