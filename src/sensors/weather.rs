use super::{Sensor, SensorReading};
use serde::Deserialize;
use std::sync::{Arc, RwLock};
use std::time::Duration;

// ---------------------------------------------------------------------------
// WMO weather interpretation codes → human-readable description
// ---------------------------------------------------------------------------

fn wmo_description(code: u16) -> &'static str {
    match code {
        0 => "Clear sky",
        1 => "Mainly clear",
        2 => "Partly cloudy",
        3 => "Overcast",
        45 => "Fog",
        48 => "Icy fog",
        51 => "Light drizzle",
        53 => "Moderate drizzle",
        55 => "Dense drizzle",
        56 => "Light freezing drizzle",
        57 => "Heavy freezing drizzle",
        61 => "Slight rain",
        63 => "Moderate rain",
        65 => "Heavy rain",
        66 => "Light freezing rain",
        67 => "Heavy freezing rain",
        71 => "Slight snow",
        73 => "Moderate snow",
        75 => "Heavy snow",
        77 => "Snow grains",
        80 => "Slight showers",
        81 => "Moderate showers",
        82 => "Heavy showers",
        85 => "Slight snow showers",
        86 => "Heavy snow showers",
        95 => "Thunderstorm",
        96 => "Thunderstorm with slight hail",
        99 => "Thunderstorm with heavy hail",
        _ => "Unknown",
    }
}

fn wind_direction_abbrev(degrees: f64) -> &'static str {
    let idx = (((degrees + 22.5) / 45.0) as usize) % 8;
    ["N", "NE", "E", "SE", "S", "SW", "W", "NW"][idx]
}

// ---------------------------------------------------------------------------
// API response types (Open-Meteo)
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, Clone)]
struct GeoResult {
    name: String,
    #[serde(default)]
    admin1: Option<String>,
    #[serde(default)]
    admin2: Option<String>,
    #[serde(default)]
    admin3: Option<String>,
    latitude: f64,
    longitude: f64,
}

#[derive(Debug, Deserialize)]
struct GeoResponse {
    results: Option<Vec<GeoResult>>,
}

#[derive(Debug, Deserialize)]
struct CurrentWeather {
    temperature_2m: f64,
    windspeed_10m: f64,
    winddirection_10m: f64,
    weathercode: f64,
    is_day: f64,
    precipitation: f64,
}

#[derive(Debug, Deserialize)]
struct CurrentUnits {
    temperature_2m: String,
    windspeed_10m: String,
    precipitation: String,
}

#[derive(Debug, Deserialize)]
struct DailyWeather {
    temperature_2m_max: Vec<f64>,
    temperature_2m_min: Vec<f64>,
    windspeed_10m_max: Vec<f64>,
    winddirection_10m_dominant: Vec<f64>,
    weathercode: Vec<f64>,
    precipitation_sum: Vec<f64>,
}

#[derive(Debug, Deserialize)]
struct DailyUnits {
    temperature_2m_max: String,
    windspeed_10m_max: String,
    precipitation_sum: String,
}

#[derive(Debug, Deserialize)]
struct ForecastResponse {
    current: Option<CurrentWeather>,
    current_units: Option<CurrentUnits>,
    daily: Option<DailyWeather>,
    daily_units: Option<DailyUnits>,
}

// ---------------------------------------------------------------------------
// Shared state
// ---------------------------------------------------------------------------

#[derive(Debug, Default, Clone)]
pub struct WeatherData {
    // location
    pub city: String,
    pub state: String,
    pub county: String,
    pub town: String,
    // conditions
    pub wmo_code: u16,
    pub is_day: bool,
    pub temp: f64,
    pub temp_min: f64,
    pub temp_max: f64,
    pub windspeed: f64,
    pub winddir: f64,
    pub precip: f64,
    // units
    pub temp_unit: String,
    pub wind_unit: String,
    pub precip_unit: String,
}

// ---------------------------------------------------------------------------
// Background fetch task
// ---------------------------------------------------------------------------

async fn fetch_weather(
    city: String,
    country: String,
    imperial: bool,
    forecast: u8,
    state: Arc<RwLock<Option<WeatherData>>>,
) {
    let client = reqwest::Client::new();
    let mut last_city_info: Option<GeoResult> = None;

    loop {
        // 1. Geocode
        if last_city_info.is_none() {
            let url = format!(
                "https://geocoding-api.open-meteo.com/v1/search?name={}&count=1&country={}",
                urlencoding::encode(&city),
                country
            );
            if let Ok(resp) = client.get(&url).send().await {
                if let Ok(geo) = resp.json::<GeoResponse>().await {
                    last_city_info = geo.results.and_then(|mut v| {
                        if v.is_empty() {
                            None
                        } else {
                            Some(v.remove(0))
                        }
                    });
                }
            }
        }

        if let Some(ref geo) = last_city_info {
            // 2. Fetch forecast
            let temp_unit = if imperial { "fahrenheit" } else { "celsius" };
            let wind_unit = if imperial { "mph" } else { "kmh" };
            let precip_unit_str = if imperial { "inch" } else { "mm" };

            let url = if forecast == 0 {
                format!(
                    "https://api.open-meteo.com/v1/forecast?latitude={}&longitude={}&current=temperature_2m,windspeed_10m,winddirection_10m,weathercode,is_day,precipitation&temperature_unit={}&windspeed_unit={}&precipitation_unit={}",
                    geo.latitude, geo.longitude, temp_unit, wind_unit, precip_unit_str
                )
            } else {
                let _day_idx = forecast.saturating_sub(1);
                format!(
                    "https://api.open-meteo.com/v1/forecast?latitude={}&longitude={}&daily=temperature_2m_max,temperature_2m_min,windspeed_10m_max,winddirection_10m_dominant,weathercode,precipitation_sum&temperature_unit={}&windspeed_unit={}&precipitation_unit={}&forecast_days=7",
                    geo.latitude, geo.longitude, temp_unit, wind_unit, precip_unit_str
                )
            };

            if let Ok(resp) = client.get(&url).send().await {
                if let Ok(fr) = resp.json::<ForecastResponse>().await {
                    let mut data = WeatherData {
                        city: geo.name.clone(),
                        state: geo.admin1.clone().unwrap_or_default(),
                        county: geo.admin2.clone().unwrap_or_default(),
                        town: geo.admin3.clone().unwrap_or_default(),
                        ..Default::default()
                    };

                    if forecast == 0 {
                        if let Some(cur) = fr.current {
                            data.wmo_code = cur.weathercode as u16;
                            data.is_day = cur.is_day > 0.5;
                            data.temp = cur.temperature_2m;
                            data.windspeed = cur.windspeed_10m;
                            data.winddir = cur.winddirection_10m;
                            data.precip = cur.precipitation;
                        }
                        if let Some(u) = fr.current_units {
                            data.temp_unit = u.temperature_2m;
                            data.wind_unit = u.windspeed_10m;
                            data.precip_unit = u.precipitation;
                        }
                    } else {
                        let day = (forecast - 1) as usize;
                        if let Some(d) = fr.daily {
                            data.wmo_code = d.weathercode.get(day).copied().unwrap_or(0.0) as u16;
                            data.temp_max = d.temperature_2m_max.get(day).copied().unwrap_or(0.0);
                            data.temp_min = d.temperature_2m_min.get(day).copied().unwrap_or(0.0);
                            data.temp = (data.temp_max + data.temp_min) / 2.0;
                            data.windspeed = d.windspeed_10m_max.get(day).copied().unwrap_or(0.0);
                            data.winddir = d
                                .winddirection_10m_dominant
                                .get(day)
                                .copied()
                                .unwrap_or(0.0);
                            data.precip = d.precipitation_sum.get(day).copied().unwrap_or(0.0);
                        }
                        if let Some(u) = fr.daily_units {
                            data.temp_unit = u.temperature_2m_max;
                            data.wind_unit = u.windspeed_10m_max;
                            data.precip_unit = u.precipitation_sum;
                        }
                    }

                    if let Ok(mut guard) = state.write() {
                        *guard = Some(data);
                    }
                }
            }
        }

        // Wait 5 minutes before next fetch
        tokio::time::sleep(Duration::from_secs(300)).await;
    }
}

// ---------------------------------------------------------------------------
// Sensor struct
// ---------------------------------------------------------------------------

pub struct WeatherSensor {
    sensor_name: String,
    data: Arc<RwLock<Option<WeatherData>>>,
}

impl WeatherSensor {
    /// Spawns the background fetch task and returns the sensor.
    /// Must be called from within a tokio runtime context.
    pub fn new(
        city: impl Into<String>,
        country: impl Into<String>,
        imperial: bool,
        forecast: u8,
    ) -> Self {
        let city = city.into();
        let country = country.into();
        let label = if forecast == 0 {
            format!("current_weather_{}", city.to_lowercase().replace(' ', "_"))
        } else {
            format!(
                "day{}_weather_{}",
                forecast,
                city.to_lowercase().replace(' ', "_")
            )
        };

        let shared = Arc::new(RwLock::new(None::<WeatherData>));
        let shared2 = Arc::clone(&shared);

        tokio::spawn(fetch_weather(city, country, imperial, forecast, shared2));

        Self {
            sensor_name: label,
            data: shared,
        }
    }
}

impl Sensor for WeatherSensor {
    fn name(&self) -> &str {
        &self.sensor_name
    }

    fn sample(&mut self, _rate_ms: u64) -> SensorReading {
        let guard = self.data.read().ok();
        let d = guard.as_ref().and_then(|g| g.as_ref());

        let (
            city,
            state,
            county,
            town,
            wmo_code,
            wmo_desc,
            is_day,
            temp,
            temp_unit,
            temp_min,
            temp_min_unit,
            temp_max,
            temp_max_unit,
            windspeed,
            wind_unit,
            winddir,
            wind_unit2,
            winddir_abbrev,
            precip,
            precip_unit,
        ) = match d {
            None => (
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                "0".to_string(),
                "Unknown".to_string(),
                "0".to_string(),
                "0".to_string(),
                "°C".to_string(),
                "0".to_string(),
                "°C".to_string(),
                "0".to_string(),
                "°C".to_string(),
                "0".to_string(),
                "km/h".to_string(),
                "0".to_string(),
                "km/h".to_string(),
                "N".to_string(),
                "0".to_string(),
                "mm".to_string(),
            ),
            Some(w) => (
                w.city.clone(),
                w.state.clone(),
                w.county.clone(),
                w.town.clone(),
                w.wmo_code.to_string(),
                wmo_description(w.wmo_code).to_string(),
                if w.is_day { "1" } else { "0" }.to_string(),
                format!("{:.1}", w.temp),
                w.temp_unit.clone(),
                format!("{:.1}", w.temp_min),
                w.temp_unit.clone(),
                format!("{:.1}", w.temp_max),
                w.temp_unit.clone(),
                format!("{:.1}", w.windspeed),
                w.wind_unit.clone(),
                format!("{:.0}", w.winddir),
                w.wind_unit.clone(),
                wind_direction_abbrev(w.winddir).to_string(),
                format!("{:.1}", w.precip),
                w.precip_unit.clone(),
            ),
        };

        // Tokens 0-19 matching weather.js
        let fields: Vec<String> = vec![
            city,
            state,
            county,
            town,     // 0-3
            wmo_code, // 4
            wmo_desc, // 5
            is_day,   // 6  "1" or "0"
            temp,
            temp_unit, // 7-8
            temp_min,
            temp_min_unit, // 9-10
            temp_max,
            temp_max_unit, // 11-12
            windspeed,
            wind_unit, // 13-14
            winddir,
            wind_unit2,     // 15-16
            winddir_abbrev, // 17
            precip,
            precip_unit, // 18-19
        ];
        SensorReading::new(fields, 0.0, 0.0)
    }
}

// We need urlencoding for the geocoding URL
mod urlencoding {
    pub fn encode(s: &str) -> String {
        s.chars()
            .map(|c| {
                if c.is_alphanumeric() || "-_.~".contains(c) {
                    c.to_string()
                } else {
                    format!("%{:02X}", c as u32)
                }
            })
            .collect()
    }
}
