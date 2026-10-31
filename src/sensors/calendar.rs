use super::{Sensor, SensorReading};
use chrono::{Datelike, Local};

pub struct CalendarSensor;

pub fn ordinal(n: u32) -> String {
    let suffix = match n % 100 {
        11..=13 => "th",
        _ => match n % 10 {
            1 => "st",
            2 => "nd",
            3 => "rd",
            _ => "th",
        },
    };
    format!("{n}{suffix}")
}

pub const WEEKDAY_FULL: [&str; 7] = [
    "Sunday",
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
];
pub const MONTH_FULL: [&str; 13] = [
    "",
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

impl Sensor for CalendarSensor {
    fn name(&self) -> &str {
        "calendar"
    }

    fn sample(&mut self, _rate_ms: u64) -> SensorReading {
        let now = Local::now();
        let y = now.year();
        let m = now.month() as usize;
        let d = now.day();
        let dow = now.weekday().num_days_from_sunday() as usize;

        let short_month = &MONTH_FULL[m][..3];
        let short_day = &WEEKDAY_FULL[dow][..3];

        SensorReading::new(
            vec![
                format!("{m}/{d}/{:02}", y % 100), // {0}  M/D/YY
                format!("{y}-{m}-{d}"),            // {1}  YYYY-M-D
                d.to_string(),                     // {2}  day number
                ordinal(d),                        // {3}  ordinal
                WEEKDAY_FULL[dow].to_string(),     // {4}  full weekday
                short_day.to_string(),             // {5}  short weekday
                MONTH_FULL[m].to_string(),         // {6}  full month
                short_month.to_string(),           // {7}  short month
                y.to_string(),                     // {8}  4-digit year
                m.to_string(),                     // {9}  month number
                format!("{:02}", y % 100),         // {10} 2-digit year
                format!("{y}.{m}.{d}"),            // {11} YYYY.M.D
            ],
            0.0,
            0.0,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordinal_suffixes() {
        assert_eq!(ordinal(1), "1st");
        assert_eq!(ordinal(2), "2nd");
        assert_eq!(ordinal(3), "3rd");
        assert_eq!(ordinal(4), "4th");
        assert_eq!(ordinal(11), "11th");
        assert_eq!(ordinal(12), "12th");
        assert_eq!(ordinal(13), "13th");
        assert_eq!(ordinal(21), "21st");
        assert_eq!(ordinal(22), "22nd");
    }

    #[test]
    fn sample_returns_twelve_fields() {
        let mut s = CalendarSensor;
        let r = s.sample(500);
        assert_eq!(r.fields.len(), 12);
        // field 8 is the 4-digit year — must be 4 chars
        assert_eq!(r.fields[8].len(), 4);
    }
}
