//! AceMagic S1 LCD/LED panel controller — single-binary Rust port of s1panel.
//!
//! Usage:
//!   acemagic-s1 [--config <path>]   (default: config.toml in CWD)

mod config;
mod error;
mod hardware;
mod render;
mod sensors;
mod app;
mod api;

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing_subscriber::{EnvFilter, fmt};

use crate::app::{AppState, DrawLoop, LcdCommand, lcd_worker, led_worker, load_config, load_theme};
use crate::config::{AppConfig, SensorInstanceConfig};
use crate::hardware::led::LedCommand;
use crate::sensors::{Sensor, SensorMap};

const DEFAULT_CONFIG: &str = "config.toml";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // ── Tracing ──────────────────────────────────────────────────────────────
    fmt()
        .with_env_filter(EnvFilter::from_default_env()
            .add_directive("acemagic_s1=info".parse().unwrap()))
        .init();

    // ── Config ───────────────────────────────────────────────────────────────
    let config_path = std::env::args()
        .skip_while(|a| a != "--config")
        .nth(1)
        .unwrap_or_else(|| DEFAULT_CONFIG.to_string());

    let cfg = match load_config(&config_path) {
        Ok(c) => { tracing::info!("Loaded config from {config_path}"); c }
        Err(e) => {
            tracing::warn!("Could not load {config_path}: {e} — using defaults");
            crate::config::AppConfig::default()
        }
    };

    let theme = match load_theme(&cfg.theme.file) {
        Ok(t) => { tracing::info!("Loaded theme from {}", cfg.theme.file); t }
        Err(e) => {
            tracing::warn!("Could not load theme {}: {e}", cfg.theme.file);
            crate::config::Theme {
                orientation: "landscape".into(),
                refresh: "redraw".into(),
                screens: vec![],
            }
        }
    };

    // ── Hardware channels ────────────────────────────────────────────────────
    let (lcd_tx, mut lcd_rx) = mpsc::channel::<LcdCommand>(64);
    let (led_tx, mut led_rx) = mpsc::channel::<LedCommand>(16);

    // ── LCD hardware worker ───────────────────────────────────────────────────
    let lcd_path = cfg.device.lcd.clone();
    let portrait = theme.is_portrait();

    let lcd_result = tokio::task::spawn_blocking(move || {
        let api = hidapi::HidApi::new()?;
        crate::hardware::lcd::LcdDevice::open(&api, &lcd_path)
    }).await?;

    match lcd_result {
        Ok(lcd) => {
            tracing::info!("LCD device opened");
            if let Err(e) = lcd.set_orientation(portrait) {
                tracing::warn!("LCD orientation: {e}");
            }
            let _ = lcd.heartbeat();
            tokio::task::spawn_blocking(move || lcd_worker(lcd_rx, lcd));
        }
        Err(e) => {
            tracing::warn!("LCD not available: {e} — running headless");
            tokio::spawn(async move { while lcd_rx.recv().await.is_some() {} });
        }
    }

    // ── LED hardware worker ───────────────────────────────────────────────────
    let led_path = cfg.device.led.clone();

    let led_result = tokio::task::spawn_blocking(move || {
        crate::hardware::led::LedDevice::open(&led_path)
    }).await?;

    match led_result {
        Ok(led) => {
            tracing::info!("LED device opened");
            tokio::task::spawn_blocking(move || led_worker(led_rx, led));
        }
        Err(e) => {
            tracing::warn!("LED not available: {e} — running without LED strip");
            tokio::spawn(async move { while led_rx.recv().await.is_some() {} });
        }
    }

    // ── App state ────────────────────────────────────────────────────────────
    let sensors = build_sensor_map(&cfg);
    tracing::info!("Loaded {} sensor(s)", sensors.len());
    let state: Arc<AppState> = AppState::new(cfg, theme, sensors, lcd_tx, led_tx).await;

    // ── Draw loop ─────────────────────────────────────────────────────────────
    let draw_state = state.clone();
    tokio::spawn(async move {
        DrawLoop::new(draw_state).run().await;
    });

    // ── REST API ──────────────────────────────────────────────────────────────
    let listen_addr = state.config.read().await.web.listen.clone();
    let router = crate::api::build_router(state.clone());
    let listener = tokio::net::TcpListener::bind(&listen_addr).await?;
    tracing::info!("REST API listening on http://{listen_addr}");

    axum::serve(listener, router)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

async fn shutdown_signal() {
    tokio::signal::ctrl_c().await.expect("failed to install Ctrl+C handler");
    tracing::info!("Received shutdown signal");
}

fn build_sensor_map(cfg: &AppConfig) -> SensorMap {
    use crate::sensors::clock::ClockSensor;
    use crate::sensors::calendar::CalendarSensor;
    use crate::sensors::cpu_usage::CpuUsageSensor;
    use crate::sensors::cpu_temp::CpuTempSensor;
    use crate::sensors::cpu_power::CpuPowerSensor;
    use crate::sensors::memory::MemorySensor;
    use crate::sensors::network::NetworkSensor;
    use crate::sensors::space::SpaceSensor;
    use crate::sensors::weather::WeatherSensor;

    let mut map: SensorMap = HashMap::new();

    for sc in &cfg.sensors {
        let name = param_str(sc, "name").unwrap_or_else(|| sc.sensor_type.clone());
        let max_points = param_usize(sc, "max_points", 300);

        let sensor: Box<dyn Sensor> = match sc.sensor_type.as_str() {
            "clock"    => Box::new(ClockSensor),
            "calendar" => Box::new(CalendarSensor),
            "cpu_usage" => Box::new(CpuUsageSensor::new(max_points)),
            "cpu_power" => Box::new(CpuPowerSensor::new(max_points)),
            "memory"    => Box::new(MemorySensor::new(max_points)),
            "cpu_temp"  => {
                let fahrenheit = param_bool(sc, "fahrenheit", false);
                Box::new(CpuTempSensor::new(max_points, fahrenheit))
            }
            "network" => {
                let iface = param_str(sc, "interface").unwrap_or_else(|| "eth0".into());
                Box::new(NetworkSensor::new(iface, max_points))
            }
            "space" => {
                let mount = param_str(sc, "mount").unwrap_or_else(|| "/".into());
                Box::new(SpaceSensor::new(&name, mount, max_points))
            }
            "weather" => {
                let city    = param_str(sc, "city").unwrap_or_else(|| "London".into());
                let country = param_str(sc, "country").unwrap_or_else(|| "GB".into());
                let imperial = param_bool(sc, "imperial", false);
                let forecast = param_usize(sc, "forecast", 0) as u8;
                Box::new(WeatherSensor::new(city, country, imperial, forecast))
            }
            t => { tracing::warn!("Unknown sensor type: {t}"); continue; }
        };

        map.insert(name, sensor);
    }

    map
}

fn param_str(sc: &SensorInstanceConfig, key: &str) -> Option<String> {
    sc.params.get(key)?.as_str().map(|s| s.to_owned())
}

fn param_usize(sc: &SensorInstanceConfig, key: &str, default: usize) -> usize {
    sc.params.get(key)
        .and_then(|v| v.as_integer())
        .map(|n| n as usize)
        .unwrap_or(default)
}

fn param_bool(sc: &SensorInstanceConfig, key: &str, default: bool) -> bool {
    sc.params.get(key)
        .and_then(|v| v.as_bool())
        .unwrap_or(default)
}
