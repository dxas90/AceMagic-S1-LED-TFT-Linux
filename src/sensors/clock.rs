use super::{Sensor, SensorReading};
use chrono::Timelike;

pub struct ClockSensor;

impl Sensor for ClockSensor {
    fn name(&self) -> &str {
        "clock"
    }

    fn sample(&mut self, _rate_ms: u64) -> SensorReading {
        let now = chrono::Local::now();
        let h24 = now.hour();
        let min = now.minute();
        let sec = now.second();
        let h12 = if h24.is_multiple_of(12) { 12 } else { h24 % 12 };
        let ampm = if h24 < 12 { "am" } else { "pm" };

        SensorReading::new(
            vec![
                format!("{}:{:02}", h24, min),               // {0} H:MM 24h
                format!("{}:{:02}", h12, min),               // {1} H:MM 12h
                format!("{:02}", sec),                       // {2} SS
                ampm.to_string(),                            // {3} am/pm
                format!("{:02}:{:02}:{:02}", h24, min, sec), // {4} HH:MM:SS
            ],
            0.0,
            0.0,
        )
    }
}
