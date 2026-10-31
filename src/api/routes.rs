//! Axum route handlers for all REST endpoints.
#![allow(dead_code)]

use std::sync::Arc;
use std::time::Duration;
use axum::{
    extract::{Path, State, Json},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::app::{AppState, LcdCommand, load_config, save_config, load_theme};
use crate::hardware::led::LedCommand;
use crate::config::{Screen, WidgetConfig};

// ── Error helper ──────────────────────────────────────────────────────────────

struct ApiError(String);
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": self.0}))).into_response()
    }
}
impl<E: std::fmt::Display> From<E> for ApiError {
    fn from(e: E) -> Self { ApiError(e.to_string()) }
}

// ── Health ────────────────────────────────────────────────────────────────────

pub async fn health() -> impl IntoResponse {
    Json(json!({
        "version": env!("CARGO_PKG_VERSION"),
        "alive": true,
    }))
}

pub async fn get_version() -> impl IntoResponse {
    Json(json!({ "version": env!("CARGO_PKG_VERSION") }))
}

// ── Config ────────────────────────────────────────────────────────────────────

pub async fn get_config(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let config = state.config.read().await;
    Json(serde_json::to_value(&*config).unwrap_or_default())
}

#[derive(Deserialize)]
pub struct SaveConfigPayload(Value);

pub async fn save_config_route(
    State(state): State<Arc<AppState>>,
    Json(body): Json<Value>,
) -> impl IntoResponse {
    let new_config: crate::config::AppConfig = match serde_json::from_value(body) {
        Ok(c) => c,
        Err(e) => return (StatusCode::BAD_REQUEST, Json(json!({"error": e.to_string()}))).into_response(),
    };
    let path = {
        let cfg = state.config.read().await;
        // Use the same path the binary was launched with; default to config.toml
        "config.toml".to_string()
    };
    if let Err(e) = save_config(&new_config, &path) {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))).into_response();
    }
    *state.config.write().await = new_config;
    state.unsaved_changes.store(false, std::sync::atomic::Ordering::SeqCst);
    Json(json!({"ok": true})).into_response()
}

pub async fn revert_config(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match load_config("config.toml") {
        Ok(cfg) => {
            *state.config.write().await = cfg;
            state.unsaved_changes.store(false, std::sync::atomic::Ordering::SeqCst);
            Json(json!({"ok": true})).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))).into_response(),
    }
}

// ── Theme ─────────────────────────────────────────────────────────────────────

pub async fn get_theme(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let theme = state.theme.read().await;
    Json(serde_json::to_value(&*theme).unwrap_or_default())
}

pub async fn save_theme(
    State(state): State<Arc<AppState>>,
    Json(body): Json<Value>,
) -> impl IntoResponse {
    let new_theme: crate::config::Theme = match serde_json::from_value(body) {
        Ok(t) => t,
        Err(e) => return (StatusCode::BAD_REQUEST, Json(json!({"error": e.to_string()}))).into_response(),
    };
    let path = state.config.read().await.theme.file.clone();
    let content = match serde_json::to_string_pretty(&new_theme) {
        Ok(s) => s,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))).into_response(),
    };
    if let Err(e) = std::fs::write(&path, content) {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))).into_response();
    }
    *state.theme.write().await = new_theme;
    state.request_redraw();
    Json(json!({"ok": true})).into_response()
}

// ── Screens ───────────────────────────────────────────────────────────────────

pub async fn list_screens(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let theme = state.theme.read().await;
    Json(serde_json::to_value(&theme.screens).unwrap_or_default())
}

pub async fn get_screen(
    State(state): State<Arc<AppState>>,
    Path(id): Path<u32>,
) -> impl IntoResponse {
    let theme = state.theme.read().await;
    match theme.screens.iter().find(|s| s.id == id) {
        Some(s) => Json(serde_json::to_value(s).unwrap_or_default()).into_response(),
        None => (StatusCode::NOT_FOUND, Json(json!({"error": "screen not found"}))).into_response(),
    }
}

pub async fn update_screen(
    State(state): State<Arc<AppState>>,
    Path(id): Path<u32>,
    Json(body): Json<Value>,
) -> impl IntoResponse {
    let new_screen: Screen = match serde_json::from_value(body) {
        Ok(s) => s,
        Err(e) => return (StatusCode::BAD_REQUEST, Json(json!({"error": e.to_string()}))).into_response(),
    };
    let mut theme = state.theme.write().await;
    match theme.screens.iter_mut().find(|s| s.id == id) {
        Some(s) => { *s = new_screen; }
        None => return (StatusCode::NOT_FOUND, Json(json!({"error": "screen not found"}))).into_response(),
    }
    state.request_redraw();
    Json(json!({"ok": true})).into_response()
}

pub async fn delete_screen(
    State(state): State<Arc<AppState>>,
    Path(id): Path<u32>,
) -> impl IntoResponse {
    let mut theme = state.theme.write().await;
    let before = theme.screens.len();
    theme.screens.retain(|s| s.id != id);
    if theme.screens.len() == before {
        return (StatusCode::NOT_FOUND, Json(json!({"error": "screen not found"}))).into_response();
    }
    Json(json!({"ok": true})).into_response()
}

pub async fn add_screen(
    State(state): State<Arc<AppState>>,
    Json(body): Json<Value>,
) -> impl IntoResponse {
    let new_screen: Screen = match serde_json::from_value(body) {
        Ok(s) => s,
        Err(e) => return (StatusCode::BAD_REQUEST, Json(json!({"error": e.to_string()}))).into_response(),
    };
    let mut theme = state.theme.write().await;
    theme.screens.push(new_screen);
    state.request_redraw();
    Json(json!({"ok": true})).into_response()
}

// ── Widgets ───────────────────────────────────────────────────────────────────

pub async fn list_widgets(
    State(state): State<Arc<AppState>>,
    Path(screen_id): Path<u32>,
) -> impl IntoResponse {
    let theme = state.theme.read().await;
    match theme.screens.iter().find(|s| s.id == screen_id) {
        Some(s) => Json(serde_json::to_value(&s.widgets).unwrap_or_default()).into_response(),
        None => (StatusCode::NOT_FOUND, Json(json!({"error": "screen not found"}))).into_response(),
    }
}

pub async fn add_widget(
    State(state): State<Arc<AppState>>,
    Path(screen_id): Path<u32>,
    Json(body): Json<Value>,
) -> impl IntoResponse {
    let new_widget: WidgetConfig = match serde_json::from_value(body) {
        Ok(w) => w,
        Err(e) => return (StatusCode::BAD_REQUEST, Json(json!({"error": e.to_string()}))).into_response(),
    };
    let mut theme = state.theme.write().await;
    match theme.screens.iter_mut().find(|s| s.id == screen_id) {
        Some(s) => { s.widgets.push(new_widget); }
        None => return (StatusCode::NOT_FOUND, Json(json!({"error": "screen not found"}))).into_response(),
    }
    state.request_redraw();
    Json(json!({"ok": true})).into_response()
}

pub async fn get_widget(
    State(state): State<Arc<AppState>>,
    Path((screen_id, widget_id)): Path<(u32, u32)>,
) -> impl IntoResponse {
    let theme = state.theme.read().await;
    let found = theme.screens.iter()
        .find(|s| s.id == screen_id)
        .and_then(|s| s.widgets.iter().find(|w| w.id == widget_id));
    match found {
        Some(w) => Json(serde_json::to_value(w).unwrap_or_default()).into_response(),
        None => (StatusCode::NOT_FOUND, Json(json!({"error": "widget not found"}))).into_response(),
    }
}

pub async fn update_widget(
    State(state): State<Arc<AppState>>,
    Path((screen_id, widget_id)): Path<(u32, u32)>,
    Json(body): Json<Value>,
) -> impl IntoResponse {
    let new_widget: WidgetConfig = match serde_json::from_value(body) {
        Ok(w) => w,
        Err(e) => return (StatusCode::BAD_REQUEST, Json(json!({"error": e.to_string()}))).into_response(),
    };
    let mut theme = state.theme.write().await;
    let updated = theme.screens.iter_mut()
        .find(|s| s.id == screen_id)
        .and_then(|s| s.widgets.iter_mut().find(|w| w.id == widget_id))
        .map(|w| { *w = new_widget; true })
        .unwrap_or(false);
    if !updated {
        return (StatusCode::NOT_FOUND, Json(json!({"error": "widget not found"}))).into_response();
    }
    state.request_redraw();
    Json(json!({"ok": true})).into_response()
}

pub async fn delete_widget(
    State(state): State<Arc<AppState>>,
    Path((screen_id, widget_id)): Path<(u32, u32)>,
) -> impl IntoResponse {
    let mut theme = state.theme.write().await;
    let screen = match theme.screens.iter_mut().find(|s| s.id == screen_id) {
        Some(s) => s,
        None => return (StatusCode::NOT_FOUND, Json(json!({"error": "screen not found"}))).into_response(),
    };
    let before = screen.widgets.len();
    screen.widgets.retain(|w| w.id != widget_id);
    if screen.widgets.len() == before {
        return (StatusCode::NOT_FOUND, Json(json!({"error": "widget not found"}))).into_response();
    }
    Json(json!({"ok": true})).into_response()
}

// ── LCD ───────────────────────────────────────────────────────────────────────

pub async fn get_lcd_screen(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match state.canvas_png().await {
        Some(png) => (
            [(axum::http::header::CONTENT_TYPE, "image/png")],
            png,
        ).into_response(),
        None => (StatusCode::INTERNAL_SERVER_ERROR, "failed to encode PNG").into_response(),
    }
}

pub async fn lcd_redraw(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    state.request_redraw();
    Json(json!({"ok": true}))
}

#[derive(Deserialize)]
pub struct OrientationPayload {
    pub portrait: bool,
}

pub async fn set_orientation(
    State(state): State<Arc<AppState>>,
    Json(body): Json<OrientationPayload>,
) -> impl IntoResponse {
    let _ = state.lcd_tx.try_send(LcdCommand::SetOrientation(body.portrait));
    Json(json!({"ok": true}))
}

// ── LED ───────────────────────────────────────────────────────────────────────

pub async fn get_led(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let cfg = state.config.read().await;
    Json(json!({
        "theme": cfg.led.theme,
        "speed": cfg.led.speed,
        "intensity": cfg.led.intensity,
    }))
}

#[derive(Deserialize)]
pub struct LedPayload {
    pub theme: u8,
    pub speed: u8,
    pub intensity: u8,
}

pub async fn set_led(
    State(state): State<Arc<AppState>>,
    Json(body): Json<LedPayload>,
) -> impl IntoResponse {
    let cmd = LedCommand { theme: body.theme, speed: body.speed, intensity: body.intensity };
    let _ = state.led_tx.try_send(cmd);
    Json(json!({"ok": true}))
}

pub async fn led_off(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let cmd = LedCommand { theme: 4, speed: 1, intensity: 1 };
    let _ = state.led_tx.try_send(cmd);
    Json(json!({"ok": true}))
}

// ── Sensors ───────────────────────────────────────────────────────────────────

pub async fn list_sensors(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let sensors = state.sensors.lock().await;
    let names: Vec<&str> = sensors.keys().map(|s| s.as_str()).collect();
    Json(json!(names))
}

pub async fn get_sensor(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    let mut sensors = state.sensors.lock().await;
    match sensors.get_mut(&name) {
        Some(sensor) => {
            let reading = sensor.sample(0, "");
            Json(json!({
                "name": name,
                "value": reading.value,
                "min": reading.min,
                "max": reading.max,
            })).into_response()
        }
        None => (StatusCode::NOT_FOUND, Json(json!({"error": "sensor not found"}))).into_response(),
    }
}

// ── Device scan ───────────────────────────────────────────────────────────────

pub async fn device_scan() -> impl IntoResponse {
    let mut result = json!({ "hid": [], "serial": [] });

    // HID scan
    if let Ok(api) = hidapi::HidApi::new() {
        let devices: Vec<Value> = api.device_list()
            .map(|d| json!({
                "path": d.path().to_string_lossy(),
                "vid": format!("{:04x}", d.vendor_id()),
                "pid": format!("{:04x}", d.product_id()),
                "manufacturer": d.manufacturer_string().unwrap_or(""),
                "product": d.product_string().unwrap_or(""),
            }))
            .collect();
        result["hid"] = json!(devices);
    }

    // Serial scan
    if let Ok(ports) = serialport::available_ports() {
        let serial: Vec<Value> = ports.iter()
            .map(|p| json!({ "name": p.port_name, "type": format!("{:?}", p.port_type) }))
            .collect();
        result["serial"] = json!(serial);
    }

    Json(result)
}

// ── Fonts ─────────────────────────────────────────────────────────────────────

pub async fn list_fonts() -> impl IntoResponse {
    // Return commonly available system fonts
    Json(json!([
        "Arial", "Liberation Sans", "DejaVu Sans",
        "Roboto", "Ubuntu", "Noto Sans",
    ]))
}

// ── Theme files ───────────────────────────────────────────────────────────────

pub async fn list_theme_files() -> impl IntoResponse {
    let mut themes: Vec<Value> = Vec::new();
    let themes_dir = std::path::Path::new("themes");
    if let Ok(entries) = std::fs::read_dir(themes_dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                if let Ok(inner) = std::fs::read_dir(&p) {
                    for f in inner.flatten() {
                        let fp = f.path();
                        if fp.extension().map_or(false, |e| e == "json") {
                            themes.push(json!({
                                "name": fp.file_stem().unwrap_or_default().to_string_lossy(),
                                "path": fp.to_string_lossy(),
                            }));
                        }
                    }
                }
            }
        }
    }
    Json(json!(themes))
}

#[derive(Deserialize)]
pub struct LoadThemePayload {
    pub path: String,
}

pub async fn load_theme_file(
    State(state): State<Arc<AppState>>,
    Json(body): Json<LoadThemePayload>,
) -> impl IntoResponse {
    match load_theme(&body.path) {
        Ok(theme) => {
            *state.theme.write().await = theme;
            state.request_redraw();
            Json(json!({"ok": true})).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))).into_response(),
    }
}

// ── Wallpapers ────────────────────────────────────────────────────────────────

pub async fn list_wallpapers() -> impl IntoResponse {
    let mut wallpapers: Vec<Value> = Vec::new();
    for dir in &["themes", "images"] {
        if let Ok(entries) = walkdir_images(dir) {
            wallpapers.extend(entries);
        }
    }
    Json(json!(wallpapers))
}

fn walkdir_images(dir: &str) -> std::io::Result<Vec<Value>> {
    let mut out = Vec::new();
    for entry in walkdir::WalkDir::new(dir).max_depth(4) {
        if let Ok(e) = entry {
            let p = e.path();
            if let Some(ext) = p.extension() {
                if matches!(ext.to_str(), Some("png") | Some("jpg") | Some("jpeg") | Some("gif")) {
                    out.push(json!({ "path": p.to_string_lossy() }));
                }
            }
        }
    }
    Ok(out)
}

pub async fn get_wallpaper(Path(name): Path<String>) -> impl IntoResponse {
    let path = format!("themes/{name}");
    match std::fs::read(&path) {
        Ok(data) => {
            let mime = if path.ends_with(".png") { "image/png" }
                else if path.ends_with(".gif") { "image/gif" }
                else { "image/jpeg" };
            ([(axum::http::header::CONTENT_TYPE, mime)], data).into_response()
        }
        Err(_) => StatusCode::NOT_FOUND.into_response(),
    }
}

// ── Icons ─────────────────────────────────────────────────────────────────────

pub async fn list_weather_icons() -> impl IntoResponse {
    Json(json!([
        "wi:day-sunny", "wi:day-cloudy", "wi:cloudy", "wi:fog",
        "wi:sprinkle", "wi:rain", "wi:snow", "wi:thunderstorm",
        "wi:storm-showers", "wi:night-clear", "wi:night-cloudy",
    ]))
}

pub async fn search_iconify() -> impl IntoResponse {
    Json(json!({ "message": "Use https://icon-sets.iconify.design to browse icons" }))
}

// ── System ────────────────────────────────────────────────────────────────────

pub async fn restart_app() -> impl IntoResponse {
    tokio::spawn(async {
        tokio::time::sleep(Duration::from_millis(200)).await;
        std::process::exit(0);
    });
    Json(json!({"ok": true, "message": "restarting"}))
}
