use super::history::History;
use super::{Sensor, SensorReading};
use std::fs;
use std::time::Instant;

pub struct NetworkSensor {
    interface: String,
    sensor_name: String,
    rx_history: History,
    tx_history: History,
    rx_pps_history: History,
    tx_pps_history: History,
    prev_rx_bytes: u64,
    prev_tx_bytes: u64,
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
            rx_history: History::new(max_points),
            tx_history: History::new(max_points),
            rx_pps_history: History::new(max_points),
            tx_pps_history: History::new(max_points),
            prev_rx_bytes: 0,
            prev_tx_bytes: 0,
            prev_rx_packets: 0,
            prev_tx_packets: 0,
            prev_time: Instant::now(),
            max_bps: 0.0,
        }
    }

    pub fn parse_net_dev_line(line: &str, iface: &str) -> Option<(u64, u64, u64, u64)> {
        let trimmed = line.trim_start();
        if !trimmed.starts_with(iface) {
            return None;
        }
        let after_colon = trimmed.split_once(':').map(|(_, r)| r)?;
        let fields: Vec<u64> = after_colon
            .split_whitespace()
            .filter_map(|s| s.parse().ok())
            .collect();
        Some((
            fields.first().copied().unwrap_or(0), // rx_bytes
            fields.get(1).copied().unwrap_or(0),  // rx_packets
            fields.get(8).copied().unwrap_or(0),  // tx_bytes
            fields.get(9).copied().unwrap_or(0),  // tx_packets
        ))
    }

    fn read_net_dev(&self) -> Option<(u64, u64, u64, u64)> {
        let content = fs::read_to_string("/proc/net/dev").ok()?;
        content
            .lines()
            .find_map(|l| Self::parse_net_dev_line(l, &self.interface))
    }

    fn read_link_speed(&self) -> u64 {
        let path = format!("/sys/class/net/{}/speed", self.interface);
        fs::read_to_string(&path)
            .ok()
            .and_then(|s| s.trim().parse::<u64>().ok())
            .map(|mbps| mbps * 1_000_000 / 8)
            .unwrap_or(0)
    }

    fn read_mtu(&self) -> u64 {
        let path = format!("/sys/class/net/{}/mtu", self.interface);
        fs::read_to_string(&path)
            .ok()
            .and_then(|s| s.trim().parse().ok())
            .unwrap_or(1500)
    }

    pub fn fmt_rate(bps: f64) -> String {
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

    fn sample(&mut self, _rate_ms: u64) -> SensorReading {
        let now = Instant::now();
        let elapsed = now.duration_since(self.prev_time).as_secs_f64().max(0.001);

        let (rx_bytes, rx_packets, tx_bytes, tx_packets) =
            self.read_net_dev().unwrap_or((0, 0, 0, 0));

        let rx_bps = rx_bytes.saturating_sub(self.prev_rx_bytes) as f64 / elapsed;
        let tx_bps = tx_bytes.saturating_sub(self.prev_tx_bytes) as f64 / elapsed;
        let rx_pps = rx_packets.saturating_sub(self.prev_rx_packets) as f64 / elapsed;
        let tx_pps = tx_packets.saturating_sub(self.prev_tx_packets) as f64 / elapsed;

        let link_cap = self.read_link_speed() as f64;
        let observed = rx_bps.max(tx_bps);
        self.max_bps = (self.max_bps.max(observed * 1.5)).min(if link_cap > 0.0 {
            link_cap
        } else {
            1_000_000_000.0
        });

        self.prev_rx_bytes = rx_bytes;
        self.prev_tx_bytes = tx_bytes;
        self.prev_rx_packets = rx_packets;
        self.prev_tx_packets = tx_packets;
        self.prev_time = now;

        self.rx_history.push(rx_bps);
        self.tx_history.push(tx_bps);
        self.rx_pps_history.push(rx_pps);
        self.tx_pps_history.push(tx_pps);

        let mtu = self.read_mtu();
        let link_mbps = (self.read_link_speed() * 8 / 1_000_000).to_string();

        // Tokens matching network.js (0-18)
        SensorReading::new(
            vec![
                self.interface.clone(),         // {0}  interface
                rx_bytes.to_string(),           // {1}  rx bytes
                self.rx_history.join_csv(),     // {2}  rx history
                Self::fmt_rate(rx_bps * 8.0),   // {3}  rx rate formatted
                tx_bytes.to_string(),           // {4}  tx bytes
                self.tx_history.join_csv(),     // {5}  tx history
                Self::fmt_rate(tx_bps * 8.0),   // {6}  tx rate formatted
                rx_packets.to_string(),         // {7}  rx packets
                format!("{:.0}", rx_pps),       // {8}  rx pps
                self.rx_pps_history.join_csv(), // {9}  rx pps history
                tx_packets.to_string(),         // {10} tx packets
                format!("{:.0}", tx_pps),       // {11} tx pps
                self.tx_pps_history.join_csv(), // {12} tx pps history
                link_mbps,                      // {13} link Mbit/s
                mtu.to_string(),                // {14} MTU
                format!("{:.0}", rx_bps * 8.0), // {15} rx bps raw
                format!("{:.0}", tx_bps * 8.0), // {16} tx bps raw
                String::new(),                  // {17} ipv4 (not implemented)
                String::new(),                  // {18} ipv6 (not implemented)
            ],
            0.0,
            self.max_bps * 8.0,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_net_dev_line_valid() {
        let line = " eth0:  1000 10 0 0 0 0 0 0   500 5 0 0 0 0 0 0";
        let (rb, rp, tb, tp) = NetworkSensor::parse_net_dev_line(line, "eth0").unwrap();
        assert_eq!(rb, 1000);
        assert_eq!(rp, 10);
        assert_eq!(tb, 500);
        assert_eq!(tp, 5);
    }

    #[test]
    fn parse_net_dev_line_wrong_iface() {
        let line = " eth0: 1000 10 0 0 0 0 0 0 500 5 0 0 0 0 0 0";
        assert!(NetworkSensor::parse_net_dev_line(line, "wlan0").is_none());
    }

    #[test]
    fn fmt_rate_ranges() {
        assert!(NetworkSensor::fmt_rate(500.0).contains("bps"));
        assert!(NetworkSensor::fmt_rate(5_000.0).contains("Kbps"));
        assert!(NetworkSensor::fmt_rate(5_000_000.0).contains("Mbps"));
        assert!(NetworkSensor::fmt_rate(2_000_000_000.0).contains("Gbps"));
    }

    #[test]
    fn sample_returns_nineteen_fields() {
        let mut s = NetworkSensor::new("lo", 10);
        let r = s.sample(500);
        assert_eq!(r.fields.len(), 19);
        assert_eq!(r.fields[0], "lo");
    }
}
