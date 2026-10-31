pub mod routes;

use std::sync::Arc;
use axum::Router;
use tower_http::cors::{CorsLayer, Any};
use crate::app::AppState;

pub fn build_router(state: Arc<AppState>) -> Router {
    use axum::routing::{get, post, put, delete};
    use routes::*;

    Router::new()
        // Health
        .route("/api/health", get(health))
        // Config
        .route("/api/config", get(get_config))
        .route("/api/config", post(save_config_route))
        .route("/api/config/revert", post(revert_config))
        // Theme / screens
        .route("/api/theme", get(get_theme))
        .route("/api/theme", post(save_theme))
        .route("/api/theme/screens", get(list_screens))
        .route("/api/theme/screens/:id", get(get_screen))
        .route("/api/theme/screens/:id", put(update_screen))
        .route("/api/theme/screens/:id", delete(delete_screen))
        .route("/api/theme/screens", post(add_screen))
        // Widgets
        .route("/api/theme/screens/:id/widgets", get(list_widgets))
        .route("/api/theme/screens/:id/widgets", post(add_widget))
        .route("/api/theme/screens/:id/widgets/:wid", get(get_widget))
        .route("/api/theme/screens/:id/widgets/:wid", put(update_widget))
        .route("/api/theme/screens/:id/widgets/:wid", delete(delete_widget))
        // LCD
        .route("/api/lcd_screen", get(get_lcd_screen))
        .route("/api/lcd_redraw", post(lcd_redraw))
        .route("/api/lcd_orientation", post(set_orientation))
        // LED
        .route("/api/led", get(get_led))
        .route("/api/led", post(set_led))
        .route("/api/led/off", post(led_off))
        // Sensors
        .route("/api/sensors", get(list_sensors))
        .route("/api/sensors/:name", get(get_sensor))
        // Device scan
        .route("/api/device_scan", get(device_scan))
        // Fonts
        .route("/api/fonts", get(list_fonts))
        // Themes (file-based themes)
        .route("/api/themes", get(list_theme_files))
        .route("/api/themes/load", post(load_theme_file))
        // Wallpaper
        .route("/api/wallpapers", get(list_wallpapers))
        .route("/api/wallpapers/:name", get(get_wallpaper))
        // Icons
        .route("/api/icons/weather", get(list_weather_icons))
        .route("/api/icons/iconify", get(search_iconify))
        // System
        .route("/api/restart", post(restart_app))
        .route("/api/version", get(get_version))
        .layer(CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any))
        .with_state(state)
}
