use chrono::Timelike;
use super::{apply_format, Sensor, SensorReading};

pub struct ClockSensor;

impl Sensor for ClockSensor {
    fn name(&self) -> &str {
        "clock"
    }

    fn sample(&mut self, _rate_ms: u64, format: &str) -> SensorReading {
        let now = chrono::Local::now();
        let h24 = now.hour();
        let min = now.minute();
        let sec = now.second();
        let h12 = if h24 % 12 == 0 { 12 } else { h24 % 12 };
        let ampm = if h24 < 12 { "am" } else { "pm" };

        let f0 = format!("{}:{:02}", h24, min);        // H:MM 24h
        let f1 = format!("{}:{:02}", h12, min);        // H:MM 12h
        let f2 = format!("{:02}", sec);                // SS
        let f3 = ampm.to_string();                     // am/pm
        let f4 = format!("{:02}:{:02}:{:02}", h24, min, sec); // HH:MM:SS

        let value = apply_format(format, &[&f0, &f1, &f2, &f3, &f4]);
        SensorReading { value, min: 0.0, max: 0.0 }
    }
}
